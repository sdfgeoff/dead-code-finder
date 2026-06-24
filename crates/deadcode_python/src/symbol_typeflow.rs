use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_generics::{expr_type, field_read_type, field_type_for_receiver};
use super::symbol_rules::{callable_identity, constructor_binding};
use super::SymbolCollector;
use crate::symbol_index::{FunctionSignature, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn external_call_result_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        let receiver_type = match attribute.value.as_ref() {
            ast::Expr::Name(receiver) => types
                .get(receiver.id.as_str())
                .cloned()
                .or_else(|| self.class_object_binding(receiver.id.as_str())),
            value => field_read_type(self.available_classes, value, types),
        }?;
        receiver_type.external.then(|| TypeBinding {
            base: format!("{}.{}", receiver_type.base, attribute.attr.as_str()),
            args: Vec::new(),
            external: true,
        })
    }

    pub(super) fn local_call_return_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            if let ast::Expr::Await(await_expr) = expr {
                return self.local_call_return_binding(&await_expr.value, types);
            }
            return None;
        };
        let callee = self.resolved_call_target(&call.func, types)?;
        let signature = self
            .available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)?;
        let return_type = signature.return_type.clone()?;
        let substitutions = self.type_var_substitutions(signature, call, types);
        Some(substitute_type_vars(&return_type, &substitutions))
    }

    pub(super) fn local_call_field_read_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Attribute(attribute) = expr else {
            return None;
        };
        let receiver_type = self.local_call_return_binding(&attribute.value, types)?;
        field_type_for_receiver(
            self.available_classes,
            &receiver_type,
            attribute.attr.as_str(),
        )
    }

    pub(super) fn fluent_self_call_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        let receiver_type = self.receiver_type_for_expr(&attribute.value, types)?;
        self.fluent_method_returns_self(&receiver_type, attribute.attr.as_str())
            .then_some(receiver_type)
    }

    fn resolved_call_target(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => callable_identity(self.module, self.imports, expr)
                .or_else(|| Some(format!("{}.{}", self.module, name.id.as_str()))),
            ast::Expr::Attribute(attribute) => {
                let receiver_type = self.receiver_type_for_expr(&attribute.value, types)?;
                Some(format!(
                    "{}.{}",
                    receiver_type.base,
                    attribute.attr.as_str()
                ))
            }
            ast::Expr::Subscript(subscript) => self.resolved_call_target(&subscript.value, types),
            _ => None,
        }
    }

    fn receiver_type_for_expr(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        match expr {
            ast::Expr::Name(receiver) => types
                .get(receiver.id.as_str())
                .cloned()
                .or_else(|| self.class_object_binding(receiver.id.as_str())),
            expr => constructor_binding(self.module, self.imports, self.rules, expr)
                .or_else(|| field_read_type(self.available_classes, expr, types))
                .or_else(|| expr_type(self.available_classes, expr, types)),
        }
    }

    fn fluent_method_returns_self(&self, receiver_type: &TypeBinding, method: &str) -> bool {
        self.rules.fluent_methods.iter().any(|rule| {
            rule.methods.iter().any(|candidate| candidate == method)
                && self.is_subclass_or_same(&receiver_type.base, &rule.receiver_type)
        })
    }

    fn is_subclass_or_same(&self, concrete_type: &str, base_type: &str) -> bool {
        concrete_type == base_type || self.is_subclass(concrete_type, base_type, &mut Vec::new())
    }

    fn is_subclass(&self, concrete_type: &str, base_type: &str, visited: &mut Vec<String>) -> bool {
        if visited.iter().any(|visited| visited == concrete_type) {
            return false;
        }
        visited.push(concrete_type.to_string());
        let Some(class_info) = self
            .available_classes
            .iter()
            .find(|class_info| class_info.class == concrete_type)
        else {
            return false;
        };
        class_info
            .bases
            .iter()
            .any(|base| base.base == base_type || self.is_subclass(&base.base, base_type, visited))
    }

    fn type_var_substitutions(
        &self,
        signature: &FunctionSignature,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) -> HashMap<String, TypeBinding> {
        let mut substitutions = HashMap::new();
        let positional_offset = signature
            .parameters
            .first()
            .is_some_and(|parameter| matches!(parameter.name.as_str(), "self" | "cls"))
            as usize;
        for (position, argument) in call.arguments.args.iter().enumerate() {
            let Some(parameter) = signature.parameters.get(position + positional_offset) else {
                continue;
            };
            self.push_type_var_substitution(
                parameter.annotation.as_ref(),
                argument,
                types,
                &mut substitutions,
            );
        }
        for keyword in &call.arguments.keywords {
            let Some(name) = keyword.arg.as_ref() else {
                continue;
            };
            let Some(parameter) = signature
                .parameters
                .iter()
                .find(|parameter| parameter.name == name.as_str())
            else {
                continue;
            };
            self.push_type_var_substitution(
                parameter.annotation.as_ref(),
                &keyword.value,
                types,
                &mut substitutions,
            );
        }
        substitutions
    }

    fn push_type_var_substitution(
        &self,
        parameter_annotation: Option<&TypeBinding>,
        argument: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
        substitutions: &mut HashMap<String, TypeBinding>,
    ) {
        let Some(type_var) = type_var_from_type_argument(parameter_annotation) else {
            return;
        };
        let Some(argument_type) = self.receiver_type_for_expr(argument, types) else {
            return;
        };
        substitutions.insert(type_var.to_string(), argument_type);
    }
}

fn type_var_from_type_argument(annotation: Option<&TypeBinding>) -> Option<&str> {
    let annotation = annotation?;
    if !matches!(
        annotation.base.as_str(),
        "typing.Type" | "typing_extensions.Type" | "Type"
    ) && !annotation.base.ends_with(".Type")
    {
        return None;
    }
    annotation.args.first().map(|arg| arg.base.as_str())
}

fn substitute_type_vars(
    binding: &TypeBinding,
    substitutions: &HashMap<String, TypeBinding>,
) -> TypeBinding {
    if let Some(substitution) = substitutions.get(&binding.base) {
        return substitution.clone();
    }
    TypeBinding {
        base: binding.base.clone(),
        args: binding
            .args
            .iter()
            .map(|arg| substitute_type_vars(arg, substitutions))
            .collect(),
        external: binding.external,
    }
}

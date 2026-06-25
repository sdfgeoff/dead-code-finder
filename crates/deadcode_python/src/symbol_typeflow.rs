use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_aliases::{expand_alias_binding, type_alias_type_binding};
use super::symbol_branch_types::{
    coalesced_optional_type, compatible_branch_type, is_empty_list_expr,
    optional_list_or_empty_list_type, optional_list_with_empty_list_type,
};
use super::symbol_callable_alias::callable_alias_target;
use super::symbol_generics::{expr_type, field_read_type, field_type_for_receiver};
use super::symbol_mapping_types::is_mapping_collection;
use super::symbol_rules::{callable_identity, constructor_binding, factory_return_binding};
use super::symbol_types::type_binding_from_expr;
use super::symbol_typevars::{
    collect_type_var_substitutions, substitute_type_vars, type_object_arg,
    type_var_from_type_argument,
};
use super::SymbolCollector;
use crate::symbol_index::{FunctionSignature, ImportTarget, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn assignment_value_binding(
        &self,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        type_alias_type_binding(self.module, self.imports, value)
            .or_else(|| self.local_call_return_binding(value, types))
            .or_else(|| self.json_mapping_call_binding(value, types))
            .or_else(|| self.known_call_result_binding(value))
            .or_else(|| self.pydantic_type_adapter_binding(value, types))
            .or_else(|| self.pydantic_validation_call_binding(value, types))
            .or_else(|| factory_return_binding(self.module, self.imports, self.rules, value))
            .or_else(|| self.tuple_literal_flow_binding(value, types))
            .or_else(|| self.list_literal_flow_binding(value, types))
            .or_else(|| self.list_comprehension_flow_binding(value, types))
            .or_else(|| self.dict_comprehension_flow_binding(value, types))
            .or_else(|| self.generator_expression_flow_binding(value, types))
            .or_else(|| self.subscript_flow_binding(value, types))
            .or_else(|| expr_type(self.available_classes, value, types))
            .or_else(|| constructor_binding(self.module, self.imports, self.rules, value))
            .or_else(|| self.local_call_field_read_binding(value, types))
            .or_else(|| self.cast_or_if_expression_binding(value, types))
            .or_else(|| self.bool_or_expression_binding(value, types))
            .or_else(|| self.binop_expression_binding(value, types))
            .or_else(|| self.external_expr_binding(value, types))
            .or_else(|| self.fluent_self_call_binding(value, types))
            .or_else(|| self.external_call_result_binding(value, types))
            .or_else(|| type_binding_from_expr(self.module, self.imports, value))
    }

    pub(super) fn known_call_result_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let callable = callable_identity(self.module, self.imports, &call.func)?;
        let base = match callable.as_str() {
            "datetime.datetime.now"
            | "datetime.datetime.utcnow"
            | "datetime.datetime.fromtimestamp"
            | "datetime.datetime.strptime"
            | "datetime.datetime.combine" => "datetime.datetime",
            "datetime.date.today"
            | "datetime.date.fromtimestamp"
            | "datetime.date.fromisoformat" => "datetime.date",
            "pathlib.Path" => "pathlib.Path",
            "inspect.stack" => {
                return Some(TypeBinding {
                    base: "list".to_string(),
                    args: vec![TypeBinding {
                        base: "inspect.FrameInfo".to_string(),
                        args: Vec::new(),
                        external: true,
                    }],
                    external: false,
                });
            }
            _ => return None,
        };
        Some(TypeBinding {
            base: base.to_string(),
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
        if let Some(return_type) = self.executor_callable_return_binding(call, types) {
            return Some(return_type);
        }
        let callee = self.resolved_call_target(&call.func, types)?;
        let signature = self
            .available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)?;
        let return_type = signature
            .concrete_return_type
            .clone()
            .or_else(|| signature.return_type.clone())?;
        let substitutions = self.type_var_substitutions(signature, call, types);
        Some(expand_alias_binding(
            &substitute_type_vars(&return_type, &substitutions),
            self.available_values,
        ))
    }

    fn executor_callable_return_binding(
        &self,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        if attribute.attr.as_str() != "run_in_executor" {
            return None;
        }
        let callback = call.arguments.args.get(1)?;
        let callback_target = self.resolved_call_target(callback, types)?;
        self.available_fn_sigs
            .iter()
            .find(|signature| signature.function == callback_target)
            .and_then(|signature| signature.return_type.clone())
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

    pub(super) fn cast_or_if_expression_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        self.cast_binding(expr)
            .or_else(|| self.if_expression_binding(expr, types))
    }

    pub(super) fn bool_or_expression_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::BoolOp(bool_op) = expr else {
            return None;
        };
        if bool_op.op != ast::BoolOp::Or || bool_op.values.len() != 2 {
            return None;
        }
        let left = self.expression_flow_binding(&bool_op.values[0], types);
        if let Some(left) = &left {
            if let Some(binding) = optional_list_or_empty_list_type(left, &bool_op.values[1]) {
                return Some(binding);
            }
        }
        let right = self.expression_flow_binding(&bool_op.values[1], types);
        if let Some(right) = &right {
            if let Some(binding) = optional_list_or_empty_list_type(right, &bool_op.values[0]) {
                return Some(binding);
            }
        }
        let (left, right) = (left?, right?);
        coalesced_optional_type(&left, &right).or_else(|| coalesced_optional_type(&right, &left))
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

    pub(super) fn resolved_call_target(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => {
                let name = name.id.as_str();
                let same_module = format!("{}.{}", self.module, name);
                callable_identity(self.module, self.imports, expr)
                    .filter(|target| target != &same_module)
                    .or_else(|| {
                        callable_alias_target(
                            self.module,
                            name,
                            types,
                            self.available_values,
                            self.available_fn_sigs,
                        )
                    })
                    .or(Some(same_module))
            }
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

    pub(super) fn receiver_type_for_expr(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        match expr {
            ast::Expr::Name(receiver) => types
                .get(receiver.id.as_str())
                .cloned()
                .or_else(|| self.class_object_binding(receiver.id.as_str()))
                .or_else(|| self.external_import_binding(receiver.id.as_str())),
            expr => self
                .local_call_return_binding(expr, types)
                .or_else(|| self.pydantic_type_adapter_binding(expr, types))
                .or_else(|| constructor_binding(self.module, self.imports, self.rules, expr))
                .or_else(|| field_read_type(self.available_classes, expr, types))
                .or_else(|| expr_type(self.available_classes, expr, types)),
        }
    }

    fn cast_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        if callable_identity(self.module, self.imports, &call.func).as_deref()
            != Some("typing.cast")
        {
            return None;
        }
        let annotation = call.arguments.args.first()?;
        type_binding_from_expr(self.module, self.imports, annotation)
    }

    fn if_expression_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::If(if_expr) = expr else {
            return None;
        };
        let body = self.expression_flow_binding(&if_expr.body, types);
        let orelse = self.expression_flow_binding(&if_expr.orelse, types);
        match (body, orelse) {
            (Some(body), Some(orelse)) => compatible_branch_type(&body, &orelse),
            (Some(body), None) if is_empty_list_expr(&if_expr.orelse) => {
                optional_list_with_empty_list_type(&body)
            }
            (None, Some(orelse)) if is_empty_list_expr(&if_expr.body) => {
                optional_list_with_empty_list_type(&orelse)
            }
            _ => None,
        }
    }

    pub(super) fn expression_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        self.cast_or_if_expression_binding(expr, types)
            .or_else(|| self.bool_or_expression_binding(expr, types))
            .or_else(|| self.binop_expression_binding(expr, types))
            .or_else(|| self.local_call_return_binding(expr, types))
            .or_else(|| self.local_call_field_read_binding(expr, types))
            .or_else(|| self.mapping_method_call_binding(expr, types))
            .or_else(|| self.json_mapping_call_binding(expr, types))
            .or_else(|| self.known_call_result_binding(expr))
            .or_else(|| self.pydantic_type_adapter_binding(expr, types))
            .or_else(|| self.pydantic_validation_call_binding(expr, types))
            .or_else(|| constructor_binding(self.module, self.imports, self.rules, expr))
            .or_else(|| self.tuple_literal_flow_binding(expr, types))
            .or_else(|| self.list_literal_flow_binding(expr, types))
            .or_else(|| self.generator_expression_flow_binding(expr, types))
            .or_else(|| self.subscript_flow_binding(expr, types))
            .or_else(|| expr_type(self.available_classes, expr, types))
    }

    fn tuple_literal_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Tuple(tuple) = expr else {
            return None;
        };
        let args = tuple
            .elts
            .iter()
            .map(|element| self.expression_flow_binding(element, types))
            .collect::<Option<Vec<_>>>()?;
        Some(TypeBinding {
            base: "tuple".to_string(),
            args,
            external: false,
        })
    }

    fn mapping_method_call_binding(
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
        if !is_mapping_collection(&receiver_type.base) {
            return None;
        }
        match attribute.attr.as_str() {
            "items" => Some(TypeBinding {
                base: "list".to_string(),
                args: vec![TypeBinding {
                    base: "tuple".to_string(),
                    args: receiver_type.args,
                    external: false,
                }],
                external: false,
            }),
            "values" => Some(TypeBinding {
                base: "list".to_string(),
                args: receiver_type.args.get(1).cloned().into_iter().collect(),
                external: false,
            }),
            "get" | "setdefault" => receiver_type.args.get(1).cloned(),
            _ => None,
        }
    }

    fn fluent_method_returns_self(&self, receiver_type: &TypeBinding, method: &str) -> bool {
        self.rules.fluent_methods.iter().any(|rule| {
            rule.methods.iter().any(|candidate| candidate == method)
                && self.is_subclass_or_same(&receiver_type.base, &rule.receiver_type)
        })
    }

    pub(super) fn is_subclass_or_same(&self, concrete_type: &str, base_type: &str) -> bool {
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

    pub(super) fn type_var_substitutions(
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
        let Some(annotation) = parameter_annotation else {
            return;
        };
        let Some(argument_type) = self.receiver_type_for_expr(argument, types) else {
            return;
        };
        if let Some(type_var) = type_var_from_type_argument(annotation) {
            substitutions.insert(
                type_var.to_string(),
                type_object_arg(&argument_type).unwrap_or(argument_type),
            );
            return;
        }
        collect_type_var_substitutions(annotation, &argument_type, substitutions);
    }

    pub(super) fn external_import_binding(&self, name: &str) -> Option<TypeBinding> {
        self.imports.iter().find_map(|import| {
            if import.binding != name {
                return None;
            }
            match &import.target {
                ImportTarget::Module {
                    module,
                    external: true,
                } => Some(TypeBinding {
                    base: module.clone(),
                    args: Vec::new(),
                    external: true,
                }),
                ImportTarget::Symbol {
                    module,
                    name,
                    external: true,
                } => Some(TypeBinding {
                    base: format!("{module}.{name}"),
                    args: Vec::new(),
                    external: true,
                }),
                ImportTarget::Module {
                    external: false, ..
                }
                | ImportTarget::Symbol {
                    external: false, ..
                }
                | ImportTarget::Star { .. } => None,
            }
        })
    }
}

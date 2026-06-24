use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_generics::{expr_type, field_read_type};
use super::symbol_rules::{callable_identity, constructor_binding};
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

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
        self.available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)
            .and_then(|signature| signature.return_type.clone())
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
                let receiver_type = match attribute.value.as_ref() {
                    ast::Expr::Name(receiver) => types
                        .get(receiver.id.as_str())
                        .cloned()
                        .or_else(|| self.class_object_binding(receiver.id.as_str())),
                    value => field_read_type(self.available_classes, value, types),
                }?;
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
}

use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_generics::field_read_type;
use super::symbol_rules::callable_identity;
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
}

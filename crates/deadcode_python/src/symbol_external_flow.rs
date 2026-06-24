use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_rules::constructor_binding;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn external_expr_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        match expr {
            ast::Expr::Attribute(attribute) => {
                let receiver = self.receiver_type_for_external_chain(&attribute.value, types)?;
                external_member_type(&receiver, attribute.attr.as_str())
            }
            ast::Expr::Call(call) => {
                let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
                    return self
                        .known_call_result_binding(expr)
                        .or_else(|| {
                            constructor_binding(self.module, self.imports, self.rules, expr)
                        })
                        .filter(|binding| binding.external);
                };
                let receiver = self.receiver_type_for_external_chain(&attribute.value, types)?;
                external_member_type(&receiver, attribute.attr.as_str())
            }
            _ => None,
        }
    }

    fn receiver_type_for_external_chain(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        self.external_expr_binding(expr, types)
            .or_else(|| self.receiver_type_for_expr(expr, types))
    }
}

fn external_member_type(receiver: &TypeBinding, member: &str) -> Option<TypeBinding> {
    receiver.external.then(|| TypeBinding {
        base: format!("{}.{}", receiver.base, member),
        args: Vec::new(),
        external: true,
    })
}

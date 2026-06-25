use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_generics::field_read_type;
use super::symbol_rules::constructor_binding;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn external_call_result_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            if let ast::Expr::Await(await_expr) = expr {
                return self.external_call_result_binding(&await_expr.value, types);
            }
            return None;
        };
        if let ast::Expr::Name(name) = call.func.as_ref() {
            let callee_type = types
                .get(name.id.as_str())
                .cloned()
                .or_else(|| self.external_import_binding(name.id.as_str()))?;
            return callee_type.external.then(|| TypeBinding {
                base: format!("{}.__call__", callee_type.base),
                args: Vec::new(),
                external: true,
            });
        }
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        let receiver_type = match attribute.value.as_ref() {
            ast::Expr::Name(receiver) => types
                .get(receiver.id.as_str())
                .cloned()
                .or_else(|| self.class_object_binding(receiver.id.as_str()))
                .or_else(|| self.external_import_binding(receiver.id.as_str())),
            value => field_read_type(self.available_classes, value, types),
        }?;
        let receiver_type = non_none_union_member(&receiver_type)
            .cloned()
            .unwrap_or(receiver_type);
        receiver_type.external.then(|| TypeBinding {
            base: format!("{}.{}", receiver_type.base, attribute.attr.as_str()),
            args: Vec::new(),
            external: true,
        })
    }

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

fn non_none_union_member(binding: &TypeBinding) -> Option<&TypeBinding> {
    if !is_union_type(&binding.base) {
        return None;
    }
    let mut non_none = binding.args.iter().filter(|arg| !is_none_type(&arg.base));
    let member = non_none.next()?;
    non_none.next().is_none().then_some(member)
}

fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "typing.Optional" | "Union" | "Optional"
    ) || type_name.ends_with(".Union")
        || type_name.ends_with(".Optional")
}

fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "NoneType" | "types.NoneType")
}

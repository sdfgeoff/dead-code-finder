use std::collections::HashMap;

use ruff_python_ast as ast;

use super::super::AccessKind;
use super::symbol_aliases::expand_alias_binding;
use super::symbol_generics::{expr_type, member_reference_target_bases};
use super::symbol_members::push_member_reference;
use super::SymbolCollector;
use crate::symbol_index::{ImportTarget, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn collect_member_reference(
        &mut self,
        owner: &str,
        attribute: &ast::ExprAttribute,
        access: AccessKind,
        types: &HashMap<String, TypeBinding>,
    ) {
        let receiver_type = match attribute.value.as_ref() {
            ast::Expr::Name(receiver) => {
                let receiver_name = receiver.id.as_str();
                if self.imports.iter().any(|import| {
                    import.binding == receiver_name && import_target_is_external(&import.target)
                }) {
                    return;
                }
                types
                    .get(receiver_name)
                    .cloned()
                    .or_else(|| self.class_object_binding(receiver_name))
            }
            value @ ast::Expr::Call(_) => self
                .local_call_return_binding(value, types)
                .or_else(|| expr_type(self.available_classes, value, types)),
            value => expr_type(self.available_classes, value, types)
                .or_else(|| self.local_call_return_binding(value, types)),
        };
        if let Some(receiver_type) = receiver_type {
            let receiver_type = expand_alias_binding(&receiver_type, self.available_values);
            if receiver_type.external {
                return;
            }
            for target_base in member_reference_target_bases(&receiver_type) {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    owner,
                    format!("{}.{}", target_base, attribute.attr.as_str()),
                    access.clone(),
                    attribute.range,
                );
            }
        } else {
            let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
                return;
            };
            self.unresolved_receivers
                .push(crate::symbol_index::UnresolvedReceiver {
                    from: owner.to_string(),
                    receiver: receiver.id.as_str().to_string(),
                    member: attribute.attr.as_str().to_string(),
                    span: self
                        .locator
                        .span_from_range_string(self.file, attribute.range),
                });
        }
    }
}

fn import_target_is_external(target: &ImportTarget) -> bool {
    match target {
        ImportTarget::Module { external, .. }
        | ImportTarget::Symbol { external, .. }
        | ImportTarget::Star { external, .. } => *external,
    }
}

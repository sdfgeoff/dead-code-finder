use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use super::symbol_generics::member_reference_target_bases;
use super::symbol_members::push_member_reference;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, ClassFieldInfo, ClassInfo, FieldAnnotation, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn typed_dict_get_call_binding(
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
        if attribute.attr.as_str() != "get" {
            return None;
        }
        let key = call
            .arguments
            .args
            .first()
            .and_then(super::symbol_expr::string_literal)?;
        let receiver_type = self.expression_flow_binding(&attribute.value, types)?;
        self.typed_dict_key_binding(&receiver_type, key)
    }

    pub(super) fn collect_typed_dict_key_reference(
        &mut self,
        owner: &str,
        receiver_type: &TypeBinding,
        key: &str,
        range: TextRange,
    ) {
        for base in member_reference_target_bases(receiver_type) {
            if !class_derives_from_any(
                self.available_classes,
                &base,
                &["typing.TypedDict", "typing_extensions.TypedDict"],
            ) {
                continue;
            }
            let Some(target) = typed_dict_key_target(self.available_classes, &base, key) else {
                continue;
            };
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                target,
                AccessKind::Read,
                range,
            );
        }
    }

    pub(super) fn collect_typed_dict_get_call_reference(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) {
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return;
        };
        if attribute.attr.as_str() != "get" {
            return;
        }
        let Some(key) = call
            .arguments
            .args
            .first()
            .and_then(super::symbol_expr::string_literal)
        else {
            return;
        };
        let Some(receiver_type) = self.expression_flow_binding(&attribute.value, types) else {
            return;
        };
        self.collect_typed_dict_key_reference(owner, &receiver_type, key, call.range);
    }

    fn typed_dict_key_binding(
        &self,
        receiver_type: &TypeBinding,
        key: &str,
    ) -> Option<TypeBinding> {
        for base in member_reference_target_bases(receiver_type) {
            if !class_derives_from_any(
                self.available_classes,
                &base,
                &["typing.TypedDict", "typing_extensions.TypedDict"],
            ) {
                continue;
            }
            let Some(field) = typed_dict_key_field(self.available_classes, &base, key) else {
                continue;
            };
            match &field.annotation {
                FieldAnnotation::Concrete(binding) => return Some(binding.clone()),
            }
        }
        None
    }
}

fn typed_dict_key_target(classes: &[ClassInfo], concrete_type: &str, key: &str) -> Option<String> {
    let class_info = classes
        .iter()
        .find(|class_info| class_info.class == concrete_type)?;
    if class_info.fields.iter().any(|field| field.name == key) {
        return Some(format!("{concrete_type}.{key}"));
    }
    class_info
        .bases
        .iter()
        .find_map(|base| typed_dict_key_target(classes, &base.base, key))
}

fn typed_dict_key_field<'a>(
    classes: &'a [ClassInfo],
    concrete_type: &str,
    key: &str,
) -> Option<&'a ClassFieldInfo> {
    let class_info = classes
        .iter()
        .find(|class_info| class_info.class == concrete_type)?;
    class_info
        .fields
        .iter()
        .find(|field| field.name == key)
        .or_else(|| {
            class_info
                .bases
                .iter()
                .find_map(|base| typed_dict_key_field(classes, &base.base, key))
        })
}

fn class_derives_from_any(classes: &[ClassInfo], concrete_type: &str, base_types: &[&str]) -> bool {
    base_types.iter().any(|base_type| {
        class_derives_from_inner(classes, concrete_type, base_type, &mut Vec::new())
    })
}

fn class_derives_from_inner(
    classes: &[ClassInfo],
    concrete_type: &str,
    base_type: &str,
    visited: &mut Vec<String>,
) -> bool {
    if visited.iter().any(|visited| visited == concrete_type) {
        return false;
    }
    visited.push(concrete_type.to_string());
    let Some(class_info) = classes
        .iter()
        .find(|class_info| class_info.class == concrete_type)
    else {
        return false;
    };
    class_info.bases.iter().any(|base| {
        base.base == base_type || class_derives_from_inner(classes, &base.base, base_type, visited)
    })
}

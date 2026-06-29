use super::symbol_generics::member_reference_target_bases;
use super::symbol_members::push_member_reference;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, ClassInfo, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn collect_typed_dict_key_reference(
        &mut self,
        owner: &str,
        receiver_type: &TypeBinding,
        key: &str,
        range: ruff_text_size::TextRange,
    ) {
        for base in member_reference_target_bases(receiver_type) {
            if !class_derives_from_any(
                self.available_classes,
                &base,
                &["typing.TypedDict", "typing_extensions.TypedDict"],
            ) {
                continue;
            }
            if !self
                .available_classes
                .iter()
                .find(|class_info| class_info.class == base)
                .is_some_and(|class_info| class_info.fields.iter().any(|field| field.name == key))
            {
                continue;
            }
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{base}.{key}"),
                AccessKind::Read,
                range,
            );
        }
    }
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

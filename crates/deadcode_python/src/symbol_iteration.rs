use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_expr::target_name;
use super::symbol_generics::{collection_item_type, iterable_item_type};
use super::symbol_members::push_member_reference;
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn iteration_item_type(
        &self,
        iter: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        self.enum_class_binding(iter).or_else(|| {
            iterable_item_type(self.available_classes, iter, types).or_else(|| {
                self.local_call_return_binding(iter, types)
                    .and_then(|binding| collection_item_type(&binding))
                    .or_else(|| {
                        self.expression_flow_binding(iter, types)
                            .and_then(|binding| collection_item_type(&binding))
                    })
            })
        })
    }

    pub(super) fn collect_enum_iteration_references(&mut self, owner: &str, iter: &ast::Expr) {
        let Some(enum_type) = self.enum_class_binding(iter) else {
            return;
        };
        for member in self.iterated_class_attribute_names(&enum_type.base) {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", enum_type.base, member),
                AccessKind::Construct,
                iter.range(),
            );
        }
    }

    fn enum_class_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        let binding = match expr {
            ast::Expr::Name(name) => self
                .class_object_binding(name.id.as_str())
                .or_else(|| type_binding_from_expr(self.module, self.imports, expr)),
            ast::Expr::Attribute(_) => type_binding_from_expr(self.module, self.imports, expr),
            _ => None,
        }?;
        self.iterated_class_is_enum(&binding.base)
            .then_some(binding)
    }

    fn iterated_class_is_enum(&self, concrete_type: &str) -> bool {
        ["enum.Enum", "enum.StrEnum", "enum.IntEnum"]
            .iter()
            .any(|base_type| {
                self.iterated_class_derives_from(concrete_type, base_type, &mut Vec::new())
            })
    }

    fn iterated_class_derives_from(
        &self,
        concrete_type: &str,
        base_type: &str,
        visited: &mut Vec<String>,
    ) -> bool {
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
        class_info.bases.iter().any(|base| {
            base.base == base_type
                || self.iterated_class_derives_from(&base.base, base_type, visited)
        })
    }

    fn iterated_class_attribute_names(&self, class_name: &str) -> Vec<String> {
        self.available_classes
            .iter()
            .find(|class_info| class_info.class == class_name)
            .map(|class_info| class_info.attributes.clone())
            .unwrap_or_default()
    }
}

pub(super) fn bind_iteration_target(
    target: &ast::Expr,
    item_type: &TypeBinding,
    types: &mut HashMap<String, TypeBinding>,
) {
    let tuple_items = match target {
        ast::Expr::Tuple(tuple) => &tuple.elts,
        ast::Expr::List(list) => &list.elts,
        _ => return,
    };
    if item_type.base != "tuple" || item_type.args.len() != tuple_items.len() {
        return;
    }
    for (target_item, binding) in tuple_items.iter().zip(&item_type.args) {
        if let Some(name) = target_name(target_item) {
            types.insert(name.to_string(), binding.clone());
        }
    }
}

pub(super) fn bind_collection_unpack_target(
    target: &ast::Expr,
    collection_type: &TypeBinding,
    types: &mut HashMap<String, TypeBinding>,
) {
    let collection_type = non_none_union_member(collection_type).unwrap_or(collection_type);
    let tuple_items = match target {
        ast::Expr::Tuple(tuple) => &tuple.elts,
        ast::Expr::List(list) => &list.elts,
        _ => return,
    };
    if is_tuple_type(&collection_type.base) && collection_type.args.len() == tuple_items.len() {
        for (target_item, binding) in tuple_items.iter().zip(&collection_type.args) {
            if let Some(name) = target_name(target_item) {
                types.insert(name.to_string(), binding.clone());
            }
        }
        return;
    }
    let Some(item_type) = collection_item_type(collection_type) else {
        return;
    };
    for target_item in tuple_items {
        if let Some(name) = target_name(target_item) {
            types.insert(name.to_string(), item_type.clone());
        }
    }
}

fn is_tuple_type(type_name: &str) -> bool {
    matches!(type_name, "tuple" | "typing.Tuple" | "Tuple") || type_name.ends_with(".tuple")
}

fn non_none_union_member(binding: &TypeBinding) -> Option<&TypeBinding> {
    if !matches!(
        binding.base.as_str(),
        "typing.Union" | "types.UnionType" | "typing.Optional" | "Optional"
    ) {
        return None;
    }
    let mut non_none = binding.args.iter().filter(|arg| !is_none_type(&arg.base));
    let member = non_none.next()?;
    non_none.next().is_none().then_some(member)
}

fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "builtins.None")
        || type_name.ends_with(".None")
        || type_name.ends_with(".NoneType")
}

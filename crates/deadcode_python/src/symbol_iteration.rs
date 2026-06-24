use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use super::symbol_generics::{collection_item_type, iterable_item_type};
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn iteration_item_type(
        &self,
        iter: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        iterable_item_type(self.available_classes, iter, types).or_else(|| {
            self.local_call_return_binding(iter, types)
                .and_then(|binding| collection_item_type(&binding))
        })
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

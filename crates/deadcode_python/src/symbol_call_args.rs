use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_rules::{callable_identity, constructor_binding};
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn concrete_argument_types(
        &self,
        arg: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Vec<String> {
        if let ast::Expr::List(list) = arg {
            return list
                .elts
                .iter()
                .filter_map(|element| {
                    constructor_binding(self.module, self.imports, self.rules, element)
                        .or_else(|| self.class_object_argument_binding(element))
                        .map(|binding| binding.base)
                })
                .collect();
        }
        constructor_binding(self.module, self.imports, self.rules, arg)
            .or_else(|| self.function_object_return_binding(arg))
            .or_else(|| self.class_object_argument_binding(arg))
            .or_else(|| self.assignment_value_binding(arg, types))
            .map(|binding| concrete_types_from_binding(&binding))
            .unwrap_or_default()
    }

    pub(super) fn keyword_argument_position(&self, callee: &str, arg: &str) -> Option<usize> {
        self.available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)?
            .parameters
            .iter()
            .position(|parameter| parameter.name == arg)
    }

    fn class_object_argument_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        match expr {
            ast::Expr::Name(name) => self.class_object_binding(name.id.as_str()),
            _ => None,
        }
    }

    fn function_object_return_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        let callable = callable_identity(self.module, self.imports, expr)?;
        self.available_fn_sigs
            .iter()
            .find(|signature| signature.function == callable)
            .and_then(|signature| {
                signature
                    .concrete_return_type
                    .clone()
                    .or_else(|| signature.return_type.clone())
            })
    }
}

fn concrete_types_from_binding(binding: &TypeBinding) -> Vec<String> {
    if matches!(binding.base.as_str(), "typing.Union" | "types.UnionType") {
        return binding
            .args
            .iter()
            .flat_map(concrete_types_from_binding)
            .collect();
    }
    if is_callable_type(&binding.base) {
        return binding
            .args
            .last()
            .map(concrete_types_from_binding)
            .unwrap_or_default();
    }
    if is_collection_type(&binding.base) {
        return binding
            .args
            .first()
            .map(concrete_types_from_binding)
            .unwrap_or_default();
    }
    vec![binding.base.clone()]
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    )
}

fn is_collection_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "list"
            | "set"
            | "tuple"
            | "typing.List"
            | "typing.Set"
            | "typing.Tuple"
            | "typing.Sequence"
            | "collections.abc.Sequence"
    )
}

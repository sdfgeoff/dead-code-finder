use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use crate::symbol_index::TypeBinding;

pub(super) fn apply_isinstance_narrowing(
    guard: &ast::Expr,
    types: &mut HashMap<String, TypeBinding>,
) {
    let Some((name, class_name)) = isinstance_guard(guard) else {
        return;
    };
    let Some(current_type) = types.get(name).cloned() else {
        return;
    };
    let Some(narrowed) = narrowed_union_member(&current_type, class_name) else {
        return;
    };
    types.insert(name.to_string(), narrowed);
}

fn isinstance_guard(guard: &ast::Expr) -> Option<(&str, &str)> {
    let ast::Expr::Call(call) = guard else {
        return None;
    };
    let ast::Expr::Name(function) = call.func.as_ref() else {
        return None;
    };
    if function.id.as_str() != "isinstance" {
        return None;
    }
    let [value, class] = &*call.arguments.args else {
        return None;
    };
    Some((target_name(value)?, class_expr_name(class)?))
}

fn class_expr_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        ast::Expr::Attribute(attribute) => Some(attribute.attr.as_str()),
        _ => None,
    }
}

fn narrowed_union_member(binding: &TypeBinding, class_name: &str) -> Option<TypeBinding> {
    if !is_union_type(&binding.base) {
        return None;
    }
    let mut matches = binding
        .args
        .iter()
        .filter(|arg| type_name_matches(&arg.base, class_name));
    let narrowed = matches.next()?;
    matches.next().is_none().then(|| narrowed.clone())
}

fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "types.UnionType" | "typing.Optional" | "Optional"
    )
}

fn type_name_matches(type_name: &str, class_name: &str) -> bool {
    type_name.rsplit('.').next().unwrap_or(type_name) == class_name
}

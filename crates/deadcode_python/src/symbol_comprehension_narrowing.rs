use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use crate::symbol_index::TypeBinding;

pub(super) fn apply_isinstance_narrowing(
    guard: &ast::Expr,
    types: &mut HashMap<String, TypeBinding>,
) {
    apply_comprehension_guard_narrowing(guard, types);
}

pub(super) fn apply_comprehension_guard_narrowing(
    guard: &ast::Expr,
    types: &mut HashMap<String, TypeBinding>,
) {
    if let ast::Expr::BoolOp(bool_op) = guard {
        if bool_op.op == ast::BoolOp::And {
            for value in &bool_op.values {
                apply_comprehension_guard_narrowing(value, types);
            }
        }
        return;
    }
    let Some((name, class_name)) = isinstance_guard(guard) else {
        apply_none_comparison_narrowing(guard, types);
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

fn apply_none_comparison_narrowing(guard: &ast::Expr, types: &mut HashMap<String, TypeBinding>) {
    let ast::Expr::Compare(compare) = guard else {
        return;
    };
    let [op] = compare.ops.as_ref() else {
        return;
    };
    if !matches!(op, ast::CmpOp::IsNot) {
        return;
    }
    let [right] = compare.comparators.as_ref() else {
        return;
    };
    if !matches!(right, ast::Expr::NoneLiteral(_)) {
        return;
    }
    let Some(name) = target_name(&compare.left) else {
        return;
    };
    let Some(current_type) = types.get(name).cloned() else {
        return;
    };
    let Some(narrowed) = union_without_none(&current_type) else {
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

fn union_without_none(binding: &TypeBinding) -> Option<TypeBinding> {
    if !is_union_type(&binding.base) {
        return (!is_none_type(&binding.base)).then(|| binding.clone());
    }
    let mut members = binding
        .args
        .iter()
        .filter(|arg| !is_none_type(&arg.base))
        .cloned()
        .collect::<Vec<_>>();
    match members.len() {
        0 => None,
        1 => members.pop(),
        _ => Some(TypeBinding {
            base: binding.base.clone(),
            args: members,
            external: binding.external,
        }),
    }
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

fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "builtins.None")
        || type_name.ends_with(".None")
        || type_name.ends_with(".NoneType")
}

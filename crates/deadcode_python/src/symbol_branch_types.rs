use ruff_python_ast as ast;

use crate::symbol_index::TypeBinding;

pub(super) fn compatible_branch_type(
    left: &TypeBinding,
    right: &TypeBinding,
) -> Option<TypeBinding> {
    if left.base == right.base && left.args == right.args && left.external == right.external {
        return Some(left.clone());
    }
    compatible_list_branch_type(left, right)
        .or_else(|| compatible_list_branch_type(right, left))
        .or_else(|| optional_branch_type(left, right))
        .or_else(|| optional_branch_type(right, left))
        .or_else(|| optional_value_branch_type(left, right))
        .or_else(|| optional_value_branch_type(right, left))
}

pub(super) fn coalesced_optional_type(
    optional: &TypeBinding,
    fallback: &TypeBinding,
) -> Option<TypeBinding> {
    optional_inner_type(optional)
        .is_some_and(|inner| inner.base == fallback.base && inner.args == fallback.args)
        .then(|| fallback.clone())
}

pub(super) fn optional_list_with_empty_list_type(binding: &TypeBinding) -> Option<TypeBinding> {
    let inner = optional_inner_type(binding)?;
    is_list_type(&inner.base).then(|| inner.clone())
}

pub(super) fn optional_list_or_empty_list_type(
    optional: &TypeBinding,
    fallback: &ast::Expr,
) -> Option<TypeBinding> {
    is_empty_list_expr(fallback).then(|| optional_list_with_empty_list_type(optional))?
}

fn compatible_list_branch_type(
    maybe_union_list: &TypeBinding,
    maybe_member_list: &TypeBinding,
) -> Option<TypeBinding> {
    if !same_list_type(&maybe_union_list.base, &maybe_member_list.base) {
        return None;
    }
    let union_item = maybe_union_list.args.first()?;
    let member_item = maybe_member_list.args.first()?;
    union_contains_type(union_item, member_item).then(|| maybe_union_list.clone())
}

fn optional_branch_type(value: &TypeBinding, maybe_none: &TypeBinding) -> Option<TypeBinding> {
    if !is_none_type(maybe_none) {
        return None;
    }
    if is_optional_type(value) {
        return Some(value.clone());
    }
    Some(TypeBinding {
        base: "typing.Optional".to_string(),
        args: vec![value.clone()],
        external: false,
    })
}

fn optional_value_branch_type(optional: &TypeBinding, value: &TypeBinding) -> Option<TypeBinding> {
    optional_inner_type(optional)
        .is_some_and(|inner| inner.base == value.base && inner.args == value.args)
        .then(|| optional.clone())
}

fn same_list_type(left: &str, right: &str) -> bool {
    is_list_type(left) && is_list_type(right)
}

fn union_contains_type(union: &TypeBinding, member: &TypeBinding) -> bool {
    matches!(union.base.as_str(), "typing.Union" | "types.UnionType")
        && union.args.iter().any(|arg| {
            arg.base == member.base && arg.args == member.args && arg.external == member.external
        })
}

fn is_optional_type(binding: &TypeBinding) -> bool {
    matches!(binding.base.as_str(), "typing.Optional" | "Optional")
}

fn is_list_type(type_name: &str) -> bool {
    matches!(type_name, "list" | "typing.List" | "List") || type_name.ends_with(".list")
}

pub(super) fn is_empty_list_expr(expr: &ast::Expr) -> bool {
    matches!(expr, ast::Expr::List(list) if list.elts.is_empty())
}

fn is_none_type(binding: &TypeBinding) -> bool {
    matches!(binding.base.as_str(), "None" | "builtins.None")
        || binding.base.ends_with(".None")
        || binding.base.ends_with(".NoneType")
}

fn optional_inner_type(binding: &TypeBinding) -> Option<&TypeBinding> {
    if is_optional_type(binding) {
        return binding.args.first();
    }
    if matches!(binding.base.as_str(), "typing.Union" | "types.UnionType") {
        let mut non_none = binding.args.iter().filter(|arg| !is_none_type(arg));
        let inner = non_none.next()?;
        if non_none.next().is_none() {
            return Some(inner);
        }
    }
    None
}

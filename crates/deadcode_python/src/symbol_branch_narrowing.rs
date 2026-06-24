use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_expr::expr_type_key;
use super::symbol_generics::expr_type;
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn branch_type_bindings(
        &self,
        test: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> (HashMap<String, TypeBinding>, HashMap<String, TypeBinding>) {
        let truthy = self.truthy_branch_type_bindings(test, types);
        let falsy = self.falsy_branch_type_bindings(test, types);
        (truthy, falsy)
    }

    fn truthy_branch_type_bindings(
        &self,
        test: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> HashMap<String, TypeBinding> {
        let mut narrowed = types.clone();
        if let Some((key, binding)) = self.isinstance_narrowing(test, types) {
            narrowed.insert(key, binding);
        } else if let Some((key, binding)) = self.none_comparison_narrowing(test, types, true) {
            narrowed.insert(key, binding);
        }
        narrowed
    }

    fn falsy_branch_type_bindings(
        &self,
        test: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> HashMap<String, TypeBinding> {
        match test {
            ast::Expr::BoolOp(bool_op) if bool_op.op == ast::BoolOp::Or => {
                let mut narrowed = types.clone();
                for value in &bool_op.values {
                    narrowed = self.falsy_branch_type_bindings(value, &narrowed);
                }
                narrowed
            }
            _ => {
                let mut narrowed = types.clone();
                if let Some((key, binding)) = self.not_isinstance_narrowing(test, types) {
                    narrowed.insert(key, binding);
                } else if let Some((key, binding)) =
                    self.none_comparison_narrowing(test, types, false)
                {
                    narrowed.insert(key, binding);
                }
                narrowed
            }
        }
    }

    fn isinstance_narrowing(
        &self,
        test: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<(String, TypeBinding)> {
        let (value, class_type) = self.isinstance_parts(test)?;
        let current = expand_alias_binding(
            &self.narrowable_expr_type(value, types)?,
            self.available_values,
        );
        let narrowed = union_member_matching(&current, &class_type.base)?;
        Some((expr_type_key(value)?, narrowed))
    }

    fn not_isinstance_narrowing(
        &self,
        test: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<(String, TypeBinding)> {
        let (value, class_type) = self.isinstance_parts(test)?;
        let current = expand_alias_binding(
            &self.narrowable_expr_type(value, types)?,
            self.available_values,
        );
        let narrowed = union_without(&current, &class_type.base)?;
        Some((expr_type_key(value)?, narrowed))
    }

    fn isinstance_parts<'a>(&self, test: &'a ast::Expr) -> Option<(&'a ast::Expr, TypeBinding)> {
        let ast::Expr::Call(call) = test else {
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
        let class_type = type_binding_from_expr(self.module, self.imports, class)?;
        Some((value, class_type))
    }

    fn none_comparison_narrowing(
        &self,
        test: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
        truthy: bool,
    ) -> Option<(String, TypeBinding)> {
        let ast::Expr::Compare(compare) = test else {
            return None;
        };
        let [op] = compare.ops.as_ref() else {
            return None;
        };
        let [right] = compare.comparators.as_ref() else {
            return None;
        };
        if !matches!(right, ast::Expr::NoneLiteral(_)) {
            return None;
        }
        let keep_none = matches!(op, ast::CmpOp::Is) == truthy;
        let current = expand_alias_binding(
            &self.narrowable_expr_type(&compare.left, types)?,
            self.available_values,
        );
        let narrowed = if keep_none {
            TypeBinding::erased("None".to_string())
        } else {
            union_without_none(&current)?
        };
        Some((expr_type_key(&compare.left)?, narrowed))
    }

    fn narrowable_expr_type(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        expr_type_key(expr)
            .and_then(|key| types.get(&key).cloned())
            .or_else(|| expr_type(self.available_classes, expr, types))
    }
}

fn union_member_matching(binding: &TypeBinding, class_name: &str) -> Option<TypeBinding> {
    if !is_union_type(&binding.base) {
        return (binding.base == class_name).then(|| binding.clone());
    }
    let members = flattened_union_members(binding);
    let mut matches = members.iter().filter(|arg| arg.base == class_name);
    let narrowed = matches.next()?;
    matches.next().is_none().then(|| narrowed.clone())
}

fn union_without(binding: &TypeBinding, class_name: &str) -> Option<TypeBinding> {
    if !is_union_type(&binding.base) {
        return (binding.base != class_name).then(|| binding.clone());
    }
    union_from_members(
        flattened_union_members(binding)
            .into_iter()
            .filter(|arg| arg.base != class_name)
            .collect(),
    )
}

fn union_without_none(binding: &TypeBinding) -> Option<TypeBinding> {
    if !is_union_type(&binding.base) {
        return (!is_none_type(&binding.base)).then(|| binding.clone());
    }
    union_from_members(
        flattened_union_members(binding)
            .into_iter()
            .filter(|arg| !is_none_type(&arg.base))
            .collect(),
    )
}

fn flattened_union_members(binding: &TypeBinding) -> Vec<TypeBinding> {
    if !is_union_type(&binding.base) {
        return vec![binding.clone()];
    }
    binding
        .args
        .iter()
        .flat_map(flattened_union_members)
        .collect()
}

fn union_from_members(mut members: Vec<TypeBinding>) -> Option<TypeBinding> {
    match members.len() {
        0 => None,
        1 => members.pop(),
        _ => Some(TypeBinding {
            base: "typing.Union".to_string(),
            args: members,
            external: false,
        }),
    }
}

fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "types.UnionType" | "typing.Optional" | "Optional"
    )
}

fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "builtins.None")
        || type_name.ends_with(".None")
        || type_name.ends_with(".NoneType")
}

pub(super) fn suite_returns(body: &[ast::Stmt]) -> bool {
    body.iter().any(|statement| {
        matches!(
            statement,
            ast::Stmt::Return(_)
                | ast::Stmt::Raise(_)
                | ast::Stmt::Break(_)
                | ast::Stmt::Continue(_)
        )
    })
}

pub(super) fn merge_completed_branch_types(
    types: &mut HashMap<String, TypeBinding>,
    branches: Vec<HashMap<String, TypeBinding>>,
) {
    let Some(first) = branches.first() else {
        return;
    };
    let keys = first.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let Some(binding) = first.get(&key) else {
            continue;
        };
        if branches
            .iter()
            .all(|branch| branch.get(&key) == Some(binding))
        {
            types.insert(key, binding.clone());
        }
    }
}

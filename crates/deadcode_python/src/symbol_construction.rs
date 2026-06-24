use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_rules::constructed_type_from_callee;
use crate::config::RuleConfig;
use crate::symbol_index::{ResolvedImport, TypeBinding};

pub(super) fn constructed_type_for_call(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    callee: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<(String, bool)> {
    if let ast::Expr::Name(name) = callee {
        if let Some(binding) = types.get(name.id.as_str()) {
            if is_type_object(&binding.base) {
                return binding.args.first().map(|arg| (arg.base.clone(), true));
            }
            return None;
        }
    }
    constructed_type_from_callee(module, imports, rules, callee)
        .map(|constructor_type| (constructor_type, false))
}

fn is_type_object(type_name: &str) -> bool {
    matches!(
        type_name,
        "type" | "typing.Type" | "typing_extensions.Type" | "Type"
    ) || type_name.ends_with(".Type")
}

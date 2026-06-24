use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_expr::target_name;
use super::symbol_generics::expr_type;
use super::symbol_members::push_member_reference;
use super::symbol_rules::constructor_binding;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn bind_context_manager_optional_var(
        &self,
        optional_vars: &ast::Expr,
        context_expr: &ast::Expr,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        let (Some(name), Some(binding)) = (
            target_name(optional_vars),
            constructor_binding(self.module, self.imports, self.rules, context_expr)
                .or_else(|| expr_type(self.available_classes, context_expr, types)),
        ) else {
            return;
        };
        types.insert(name.to_string(), binding);
    }

    pub(super) fn collect_context_manager_references(
        &mut self,
        owner: &str,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        let Some(binding) = constructor_binding(self.module, self.imports, self.rules, expr)
            .or_else(|| expr_type(self.available_classes, expr, types))
        else {
            return;
        };
        for method in ["__enter__", "__exit__"] {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", binding.base, method),
                AccessKind::Call,
                expr.range(),
            );
        }
    }
}

use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use super::symbol_iteration::bind_iteration_target;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_with_statement_references(
        &mut self,
        owner: &str,
        with_stmt: &ast::StmtWith,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        for item in &with_stmt.items {
            self.collect_expr_references(owner, &item.context_expr, types);
            self.collect_context_manager_references(owner, &item.context_expr, types);
            if let Some(optional_vars) = &item.optional_vars {
                self.collect_assignment_target(owner, optional_vars, types);
                self.bind_context_manager_optional_var(optional_vars, &item.context_expr, types);
            }
        }
        for nested in &with_stmt.body {
            self.collect_statement_references(owner, nested, types);
        }
    }

    pub(super) fn collect_for_statement_references(
        &mut self,
        owner: &str,
        for_stmt: &ast::StmtFor,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        self.collect_expr_references(owner, &for_stmt.iter, types);
        self.collect_assignment_target(owner, &for_stmt.target, types);
        let item_type = self.iteration_item_type(&for_stmt.iter, types);
        if let (Some(name), Some(item_type)) = (target_name(&for_stmt.target), item_type) {
            types.insert(name.to_string(), item_type);
        } else if let Some(item_type) = self.iteration_item_type(&for_stmt.iter, types) {
            bind_iteration_target(&for_stmt.target, &item_type, types);
        }
        for nested in &for_stmt.body {
            self.collect_statement_references(owner, nested, types);
        }
        for nested in &for_stmt.orelse {
            self.collect_statement_references(owner, nested, types);
        }
    }
}

use std::collections::HashMap;

use ruff_python_ast as ast;

use super::super::AccessKind;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_assignment_target(
        &mut self,
        owner: &str,
        target: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        match target {
            ast::Expr::Attribute(attribute) => {
                self.collect_member_reference(owner, attribute, AccessKind::Write, types);
                self.collect_expr_references(owner, &attribute.value, types);
            }
            ast::Expr::Tuple(tuple) => {
                for element in &tuple.elts {
                    self.collect_assignment_target(owner, element, types);
                }
            }
            ast::Expr::List(list) => {
                for element in &list.elts {
                    self.collect_assignment_target(owner, element, types);
                }
            }
            _ => {}
        }
    }
}

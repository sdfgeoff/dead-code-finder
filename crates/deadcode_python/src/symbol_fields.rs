use deadcode_core::SymbolKind;
use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use super::super::{IndexedSymbol, SourceLocator};
use super::symbol_expr::self_attribute_name;

pub(super) fn collect_self_assignments(
    module: &str,
    file: &str,
    locator: &SourceLocator,
    symbols: &mut Vec<IndexedSymbol>,
    class_name: &str,
    body: &[ast::Stmt],
) {
    for statement in body {
        collect_self_assignments_in_statement(
            module, file, locator, symbols, class_name, statement,
        );
    }
}

fn collect_self_assignments_in_statement(
    module: &str,
    file: &str,
    locator: &SourceLocator,
    symbols: &mut Vec<IndexedSymbol>,
    class_name: &str,
    statement: &ast::Stmt,
) {
    match statement {
        ast::Stmt::Assign(assign) => {
            for target in &assign.targets {
                if let Some(name) = self_attribute_name(target) {
                    push_self_symbol(
                        module,
                        file,
                        locator,
                        symbols,
                        class_name,
                        name,
                        SymbolKind::Attribute,
                        assign.range,
                    );
                }
            }
        }
        ast::Stmt::AnnAssign(assign) => {
            if let Some(name) = self_attribute_name(&assign.target) {
                push_self_symbol(
                    module,
                    file,
                    locator,
                    symbols,
                    class_name,
                    name,
                    SymbolKind::Field,
                    assign.range,
                );
            }
        }
        ast::Stmt::If(if_stmt) => {
            for nested in &if_stmt.body {
                collect_self_assignments_in_statement(
                    module, file, locator, symbols, class_name, nested,
                );
            }
            for clause in &if_stmt.elif_else_clauses {
                for nested in &clause.body {
                    collect_self_assignments_in_statement(
                        module, file, locator, symbols, class_name, nested,
                    );
                }
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn push_self_symbol(
    module: &str,
    file: &str,
    locator: &SourceLocator,
    symbols: &mut Vec<IndexedSymbol>,
    class_name: &str,
    name: &str,
    kind: SymbolKind,
    range: TextRange,
) {
    symbols.push(IndexedSymbol {
        qualified_name: format!("{module}.{class_name}.{name}"),
        name: name.to_string(),
        kind,
        span: locator.span_from_range_string(file, range),
    });
}

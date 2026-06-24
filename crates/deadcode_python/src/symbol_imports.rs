use ruff_python_ast as ast;

pub(super) fn resolve_import_from_base(
    current_module: &str,
    import_from: &ast::StmtImportFrom,
) -> Option<String> {
    let imported_module = import_from.module.as_ref().map(ast::Identifier::as_str);
    if import_from.level == 0 {
        return imported_module.map(ToString::to_string);
    }

    let mut parts = current_module.split('.').collect::<Vec<_>>();
    parts.pop();
    let ancestor_count = import_from.level.saturating_sub(1) as usize;
    if ancestor_count > parts.len() {
        return None;
    }
    parts.truncate(parts.len() - ancestor_count);
    if let Some(imported_module) = imported_module {
        parts.extend(imported_module.split('.'));
    }
    Some(parts.join("."))
}

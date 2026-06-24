use std::collections::HashSet;

use ruff_python_ast as ast;

use super::super::{ImportTarget, ReexportMap, ResolvedImport, SourceLocator};

pub(super) fn collect_import(
    file: &str,
    locator: &SourceLocator,
    imports: &mut Vec<ResolvedImport>,
    known_modules: &HashSet<String>,
    import: &ast::StmtImport,
) {
    for alias in &import.names {
        let target_module = alias.name.as_str().to_string();
        let binding = alias
            .asname
            .as_ref()
            .map_or_else(|| first_module_segment(&target_module), ToString::to_string);
        push_import(
            file,
            locator,
            imports,
            binding,
            ImportTarget::Module {
                external: !known_modules.contains(&target_module),
                module: target_module,
            },
            import.range,
        );
    }
}

pub(super) fn collect_import_from(
    module: &str,
    file: &str,
    locator: &SourceLocator,
    imports: &mut Vec<ResolvedImport>,
    known_modules: &HashSet<String>,
    reexports: &ReexportMap,
    import_from: &ast::StmtImportFrom,
) {
    let Some(base_module) =
        resolve_import_from_base(module, import_from, is_package_init_file(file))
    else {
        return;
    };
    let base_is_external = !known_modules.contains(&base_module);
    for alias in &import_from.names {
        let imported_name = alias.name.as_str();
        let binding = alias
            .asname
            .as_ref()
            .map_or_else(|| imported_name.to_string(), ToString::to_string);
        let target = if imported_name == "*" {
            ImportTarget::Star {
                external: base_is_external,
                module: base_module.clone(),
            }
        } else {
            let candidate_module = format!("{base_module}.{imported_name}");
            if known_modules.contains(&candidate_module) {
                ImportTarget::Module {
                    external: false,
                    module: candidate_module,
                }
            } else if let Some(target) =
                reexports.get(&(base_module.clone(), imported_name.to_string()))
            {
                target.clone()
            } else {
                ImportTarget::Symbol {
                    external: base_is_external,
                    module: base_module.clone(),
                    name: imported_name.to_string(),
                }
            }
        };
        push_import(file, locator, imports, binding, target, import_from.range);
    }
}

pub(super) fn resolve_import_from_base(
    current_module: &str,
    import_from: &ast::StmtImportFrom,
    is_package_init: bool,
) -> Option<String> {
    let imported_module = import_from.module.as_ref().map(ast::Identifier::as_str);
    if import_from.level == 0 {
        return imported_module.map(ToString::to_string);
    }

    let mut parts = current_module.split('.').collect::<Vec<_>>();
    if !is_package_init {
        parts.pop();
    }
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

fn is_package_init_file(file: &str) -> bool {
    file.ends_with("/__init__.py") || file.ends_with("\\__init__.py")
}

fn push_import(
    file: &str,
    locator: &SourceLocator,
    imports: &mut Vec<ResolvedImport>,
    binding: String,
    target: ImportTarget,
    range: ruff_text_size::TextRange,
) {
    imports.push(ResolvedImport {
        binding,
        target,
        span: locator.span_from_range_string(file, range),
    });
}

fn first_module_segment(module: &str) -> String {
    module.split('.').next().unwrap_or(module).to_string()
}

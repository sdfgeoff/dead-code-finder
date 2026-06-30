use std::path::Path;

use crate::config::LoadedProjectConfig;

use super::{ModuleIndex, ReexportMap, ResolvedRouteGlob};

pub(super) fn reexport_map(modules: &[ModuleIndex]) -> ReexportMap {
    let mut reexports = ReexportMap::new();
    for module in modules {
        for import in &module.imports {
            reexports.insert(
                (module.module.clone(), import.binding.clone()),
                import.target.clone(),
            );
        }
    }
    reexports
}

pub(super) fn resolve_route_globs(
    config: &LoadedProjectConfig,
    modules: &[ModuleIndex],
) -> Vec<ResolvedRouteGlob> {
    config
        .rules
        .route_globs
        .iter()
        .map(|rule| {
            let symbols = modules
                .iter()
                .filter(|module| route_glob_matches(&config.project_dir, &rule.glob, &module.file))
                .flat_map(|module| {
                    [
                        module.module.clone(),
                        format!("{}.{}", module.module, rule.export),
                    ]
                })
                .collect();
            ResolvedRouteGlob {
                when_function_called: rule.when_function_called.clone(),
                symbols,
            }
        })
        .collect()
}

fn route_glob_matches(project_dir: &Path, pattern: &str, file: &Path) -> bool {
    let relative = file.strip_prefix(project_dir).unwrap_or(file);
    let relative = relative.to_string_lossy().replace('\\', "/");
    if let Some((prefix, suffix)) = pattern.split_once("**") {
        return relative.starts_with(prefix) && relative.ends_with(suffix.trim_start_matches('/'));
    }
    relative == pattern
}

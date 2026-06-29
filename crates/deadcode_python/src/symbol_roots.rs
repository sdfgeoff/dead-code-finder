use std::path::Path;

use deadcode_core::SymbolKind;

use crate::config::LoadedProjectConfig;
use crate::symbol_index::{IndexedSymbol, RootSymbol};

pub(crate) fn root_symbols_for_module(
    module: &str,
    symbols: &[IndexedSymbol],
    primary_root_group: &str,
    configured_root_groups: Vec<String>,
    is_test: bool,
    has_main_entrypoint: bool,
) -> Vec<RootSymbol> {
    let mut roots = Vec::new();
    if configured_root_groups.is_empty() && is_test {
        return roots;
    }
    if configured_root_groups.is_empty() && has_main_entrypoint {
        roots.push(RootSymbol {
            group: primary_root_group.to_string(),
            symbol: module.to_string(),
        });
        return roots;
    }
    for group in configured_root_groups {
        if is_test {
            roots.extend(test_function_roots(symbols, &group));
        } else {
            roots.push(RootSymbol {
                group,
                symbol: module.to_string(),
            });
        }
    }
    roots
}

pub(crate) fn root_groups_for_file(config: &LoadedProjectConfig, file: &Path) -> Vec<String> {
    let is_test = is_test_file(config, file);
    config
        .root_groups
        .iter()
        .filter(|group| {
            if group.name == "test" && is_test {
                return true;
            }
            if group.name == "weak" && is_test && !config.include_tests {
                return false;
            }
            group
                .entrypoints
                .iter()
                .any(|entrypoint| configured_path_matches(config, entrypoint, file))
        })
        .map(|group| group.name.clone())
        .collect()
}

pub(crate) fn is_test_file(config: &LoadedProjectConfig, file: &Path) -> bool {
    let relative = file.strip_prefix(&config.project_dir).unwrap_or(file);
    let relative_text = relative.to_string_lossy();
    let filename = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    config
        .test_patterns
        .iter()
        .any(|pattern| match pattern.as_str() {
            "tests/**" => relative
                .components()
                .any(|part| part.as_os_str() == "tests"),
            "test_*.py" => filename.starts_with("test_") && filename.ends_with(".py"),
            "*_test.py" => filename.ends_with("_test.py"),
            "*_test_*.py" => filename.contains("_test_") && filename.ends_with(".py"),
            "conftest.py" => filename == "conftest.py",
            pattern => relative_text == pattern,
        })
}

fn test_function_roots(symbols: &[IndexedSymbol], group: &str) -> Vec<RootSymbol> {
    symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Function && symbol.name.starts_with("test_"))
        .map(|symbol| RootSymbol {
            group: group.to_string(),
            symbol: symbol.qualified_name.clone(),
        })
        .collect()
}

fn configured_path_matches(config: &LoadedProjectConfig, pattern: &str, file: &Path) -> bool {
    if !pattern.contains('*') {
        return config.project_dir.join(pattern) == file;
    }
    let relative = file.strip_prefix(&config.project_dir).unwrap_or(file);
    let relative = relative.to_string_lossy().replace('\\', "/");
    glob_pattern_matches(pattern, &relative)
}

fn glob_pattern_matches(pattern: &str, relative: &str) -> bool {
    if pattern == "**/*.py" {
        return relative.ends_with(".py");
    }
    if let Some((prefix, suffix)) = pattern.split_once("**") {
        if !relative.starts_with(prefix) {
            return false;
        }
        let suffix = suffix.trim_start_matches('/');
        if suffix.contains('*') {
            return glob_pattern_matches(suffix, &relative[prefix.len()..]);
        }
        return relative.ends_with(suffix);
    }
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return relative.starts_with(prefix) && relative.ends_with(suffix);
    }
    relative == pattern
}

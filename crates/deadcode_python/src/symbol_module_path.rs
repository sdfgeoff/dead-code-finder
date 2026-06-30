use std::path::Path;

use crate::config::ResolvedRoot;

pub(crate) fn module_name_for_file(root: &ResolvedRoot, file: &Path) -> String {
    let relative = file.strip_prefix(&root.path).unwrap_or(file);
    let mut parts = relative
        .iter()
        .filter_map(|part| part.to_str())
        .map(strip_py_extension)
        .filter(|part| part != "__init__")
        .collect::<Vec<_>>();
    if !root.module.is_empty() {
        parts.insert(0, root.module.clone());
    }
    parts.join(".")
}

fn strip_py_extension(part: &str) -> String {
    part.strip_suffix(".pyi")
        .or_else(|| part.strip_suffix(".py"))
        .unwrap_or(part)
        .to_string()
}

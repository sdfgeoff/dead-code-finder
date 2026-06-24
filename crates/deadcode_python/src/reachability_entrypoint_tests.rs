use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{LoadedProjectConfig, ResolvedRoot, RuleConfig};
use crate::symbol_index::index_project;

use super::*;

#[test]
fn with_statement_marks_context_manager_methods_live() {
    let workspace = test_workspace("with_statement_marks_context_manager_methods_live");
    let package = workspace.join("pkg");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("main.py"),
        r#"
class Resource:
    def __enter__(self):
        pass

    def __exit__(self):
        pass

class Other:
    def __enter__(self):
        pass

    def __exit__(self):
        pass

def run():
    with Resource():
        pass

run()
"#,
    )
    .unwrap();
    let config = loaded_config(
        &workspace,
        vec![root(&package, "pkg")],
        vec!["pkg/main.py".to_string()],
    );

    let index = index_project(&config).unwrap();
    let findings = find_unused_symbols(&index);
    let symbols = finding_symbols(&findings);

    assert!(!symbols.contains(&"pkg.main.Resource.__enter__".to_string()));
    assert!(!symbols.contains(&"pkg.main.Resource.__exit__".to_string()));
    assert!(symbols.contains(&"pkg.main.Other.__enter__".to_string()));
    assert!(symbols.contains(&"pkg.main.Other.__exit__".to_string()));
}

#[test]
fn weak_entrypoints_do_not_keep_production_symbols_alive() {
    let workspace = test_workspace("weak_entrypoints_do_not_keep_production_symbols_alive");
    let package = workspace.join("pkg");
    fs::create_dir_all(package.join("scripts")).unwrap();
    fs::write(
        package.join("service.py"),
        r#"
def script_only():
    pass

def dead():
    pass
"#,
    )
    .unwrap();
    fs::write(
        package.join("main.py"),
        r#"
def entry():
    pass

entry()
"#,
    )
    .unwrap();
    fs::write(
        package.join("scripts/backfill.py"),
        r#"
from pkg.service import script_only

if __name__ == "__main__":
    script_only()
"#,
    )
    .unwrap();
    let mut config = loaded_config(
        &workspace,
        vec![root(&package, "pkg")],
        vec!["pkg/main.py".to_string()],
    );
    config.weak_entrypoints = vec!["pkg/scripts/*.py".to_string()];

    let index = index_project(&config).unwrap();
    let findings = find_unused_symbols(&index);
    let script_only = findings
        .iter()
        .find(|finding| finding.symbol == "pkg.service.script_only")
        .unwrap();
    let symbols = finding_symbols(&findings);

    assert_eq!(script_only.reachable_from, vec!["weak".to_string()]);
    assert!(symbols.contains(&"pkg.service.dead".to_string()));
}

fn finding_symbols(findings: &[deadcode_core::Finding]) -> Vec<String> {
    findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect()
}

fn loaded_config(
    workspace: &Path,
    roots: Vec<ResolvedRoot>,
    entrypoints: Vec<String>,
) -> LoadedProjectConfig {
    LoadedProjectConfig {
        config_path: workspace.join("dead-code-finder.json"),
        project_dir: workspace.to_path_buf(),
        roots,
        entrypoints,
        weak_entrypoints: Vec::new(),
        include_tests: false,
        test_patterns: Vec::new(),
        rules: RuleConfig::default(),
    }
}

fn root(path: &Path, module: &str) -> ResolvedRoot {
    ResolvedRoot {
        path: path.to_path_buf(),
        module: module.to_string(),
    }
}

fn test_workspace(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("deadcode_reachability_{name}_{unique}"));
    fs::create_dir_all(&path).unwrap();
    path
}

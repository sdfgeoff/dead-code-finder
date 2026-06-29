use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{
    ConstructorRule, LoadedProjectConfig, LoadedRootGroup, ResolvedRoot, RuleConfig,
};
use crate::symbol_index::index_project;

use super::*;

#[test]
fn class_attribute_access_and_annotations_mark_members_live() {
    let workspace = test_workspace("class_attribute_access_and_annotations_mark_members_live");
    let package = workspace.join("pkg");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("main.py"),
        r#"
from enum import Enum
from typing import Literal

class Status(Enum):
    LIVE = "live"
    ANNOTATED = "annotated"
    DEAD = "dead"

class Model:
    status: Status
    typed_status: Literal[Status.ANNOTATED]

def run():
    Model()
    return Status.LIVE

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
    let diagnostics = unresolved_receiver_diagnostics(&index);

    assert!(!symbols.contains(&"pkg.main.Status.LIVE".to_string()));
    assert!(!symbols.contains(&"pkg.main.Status.ANNOTATED".to_string()));
    assert!(symbols.contains(&"pkg.main.Status.DEAD".to_string()));
    assert!(diagnostics.is_empty());
}

#[test]
fn external_imported_symbols_do_not_emit_unresolved_receiver_diagnostics() {
    let workspace =
        test_workspace("external_imported_symbols_do_not_emit_unresolved_receiver_diagnostics");
    let package = workspace.join("pkg");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("main.py"),
        r#"
from external_sdk import client

def run():
    client.call()

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

    assert!(unresolved_receiver_diagnostics(&index).is_empty());
}

#[test]
fn external_parameter_annotations_suppress_receiver_diagnostics() {
    let workspace = test_workspace("external_parameter_annotations_suppress_receiver_diagnostics");
    let package = workspace.join("pkg");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("main.py"),
        r#"
from external_orm import Session

def run(session: Session):
    session.execute("select 1")

run(None)
"#,
    )
    .unwrap();
    let config = loaded_config(
        &workspace,
        vec![root(&package, "pkg")],
        vec!["pkg/main.py".to_string()],
    );

    let index = index_project(&config).unwrap();

    assert!(unresolved_receiver_diagnostics(&index).is_empty());
}

#[test]
fn imported_local_module_member_access_marks_symbol_live() {
    let workspace = test_workspace("imported_local_module_member_access_marks_symbol_live");
    let package = workspace.join("pkg");
    fs::create_dir_all(package.join("helpers")).unwrap();
    fs::write(package.join("helpers/__init__.py"), "").unwrap();
    fs::write(
        package.join("helpers/items.py"),
        r#"
def live():
    pass

def dead():
    pass
"#,
    )
    .unwrap();
    fs::write(
        package.join("main.py"),
        r#"
from pkg.helpers import items

def run():
    items.live()

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

    assert!(unresolved_receiver_diagnostics(&index).is_empty());
    assert!(!symbols.contains(&"pkg.helpers.items.live".to_string()));
    assert!(symbols.contains(&"pkg.helpers.items.dead".to_string()));
}

#[test]
fn configured_external_factory_type_suppresses_receiver_diagnostics() {
    let workspace =
        test_workspace("configured_external_factory_type_suppresses_receiver_diagnostics");
    let package = workspace.join("pkg");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("main.py"),
        r#"
import structlog

logger = structlog.get_logger(__name__)

def run():
    logger.info("event")

run()
"#,
    )
    .unwrap();
    let mut config = loaded_config(
        &workspace,
        vec![root(&package, "pkg")],
        vec!["pkg/main.py".to_string()],
    );
    config.rules = RuleConfig {
        constructors: vec![ConstructorRule {
            match_: "structlog.get_logger".to_string(),
            produces_type: "structlog.BoundLogger".to_string(),
        }],
        ..RuleConfig::default()
    };

    let index = index_project(&config).unwrap();

    assert!(unresolved_receiver_diagnostics(&index).is_empty());
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
        root_groups: vec![LoadedRootGroup {
            name: "main".to_string(),
            entrypoints,
            counts_as_used: true,
        }],
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

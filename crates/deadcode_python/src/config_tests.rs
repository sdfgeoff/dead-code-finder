use std::fs;
use std::path::PathBuf;

use super::*;

#[test]
fn resolves_explicit_roots() {
    let workspace = test_workspace("resolves_explicit_roots");
    fs::create_dir_all(workspace.join("example_app")).unwrap();
    let config = ProjectConfig {
        roots: vec![RootConfig {
            path: "example_app".to_string(),
            module: "example_app".to_string(),
        }],
        entrypoints: vec![],
        weak_entrypoints: vec![],
        include_tests: false,
        test_patterns: default_test_patterns(),
        rules: RuleConfig::default(),
    };

    let roots = resolve_roots(&workspace, &config).unwrap();

    assert_eq!(
        roots,
        vec![ResolvedRoot {
            path: workspace.join("example_app"),
            module: "example_app".to_string()
        }]
    );
}

#[test]
fn expands_workspace_root_globs() {
    let workspace = test_workspace("expands_workspace_root_globs");
    fs::create_dir_all(workspace.join("packages/a/src/pkg_a")).unwrap();
    fs::create_dir_all(workspace.join("packages/b/src/pkg_b")).unwrap();
    let config = ProjectConfig {
        roots: vec![RootConfig {
            path: "packages/*/src/*".to_string(),
            module: "{basename}".to_string(),
        }],
        entrypoints: vec![],
        weak_entrypoints: vec![],
        include_tests: false,
        test_patterns: default_test_patterns(),
        rules: RuleConfig::default(),
    };

    let roots = resolve_roots(&workspace, &config).unwrap();

    assert_eq!(
        roots,
        vec![
            ResolvedRoot {
                path: workspace.join("packages/a/src/pkg_a"),
                module: "pkg_a".to_string()
            },
            ResolvedRoot {
                path: workspace.join("packages/b/src/pkg_b"),
                module: "pkg_b".to_string()
            }
        ]
    );
}

#[test]
fn rejects_duplicate_modules() {
    let workspace = test_workspace("rejects_duplicate_modules");
    fs::create_dir_all(workspace.join("one")).unwrap();
    fs::create_dir_all(workspace.join("two")).unwrap();
    let config = ProjectConfig {
        roots: vec![
            RootConfig {
                path: "one".to_string(),
                module: "same".to_string(),
            },
            RootConfig {
                path: "two".to_string(),
                module: "same".to_string(),
            },
        ],
        entrypoints: vec![],
        weak_entrypoints: vec![],
        include_tests: false,
        test_patterns: default_test_patterns(),
        rules: RuleConfig::default(),
    };

    let error = resolve_roots(&workspace, &config).unwrap_err();

    assert!(matches!(error, ConfigError::DuplicateModule { module } if module == "same"));
}

#[test]
fn loads_json_config() {
    let workspace = test_workspace("loads_json_config");
    fs::create_dir_all(workspace.join("pkg")).unwrap();
    fs::write(
        workspace.join("dead-code-finder.json"),
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["main.py"],
            "weakEntrypoints": ["scripts/*.py"],
            "includeTests": true,
            "rules": {
                "classSurfaces": [{
                    "base": "pkg.orm.Base",
                    "effect": "markClassAttributes"
                }]
            }
        }"#,
    )
    .unwrap();

    let loaded = load_project_config(&workspace.join("dead-code-finder.json")).unwrap();

    assert_eq!(loaded.entrypoints, vec!["main.py"]);
    assert_eq!(loaded.weak_entrypoints, vec!["scripts/*.py"]);
    assert!(loaded.include_tests);
    assert_eq!(loaded.roots[0].module, "pkg");
    assert_eq!(loaded.rules.class_surfaces[0].base, "pkg.orm.Base");
}

#[test]
fn rejects_invalid_rule_effects() {
    let workspace = test_workspace("rejects_invalid_rule_effects");
    fs::create_dir_all(workspace.join("pkg")).unwrap();
    let config_path = workspace.join("dead-code-finder.json");
    fs::write(
        &config_path,
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "rules": {
                "decorators": [{
                    "receiverType": "framework.Router",
                    "methods": ["get"],
                    "effect": "doSomethingDynamic"
                }]
            }
        }"#,
    )
    .unwrap();

    let error = load_project_config(&config_path).unwrap_err();

    assert!(matches!(error, ConfigError::InvalidRule { .. }));
}

fn test_workspace(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("deadcode_config_{name}_{unique}"));
    fs::create_dir_all(&path).unwrap();
    path
}

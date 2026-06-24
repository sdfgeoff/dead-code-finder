use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub roots: Vec<RootConfig>,
    #[serde(default)]
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub include_tests: bool,
    #[serde(default = "default_test_patterns")]
    pub test_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootConfig {
    pub path: String,
    pub module: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProjectConfig {
    pub config_path: PathBuf,
    pub project_dir: PathBuf,
    pub roots: Vec<ResolvedRoot>,
    pub entrypoints: Vec<String>,
    pub include_tests: bool,
    pub test_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRoot {
    pub path: PathBuf,
    pub module: String,
}

#[derive(Debug)]
pub enum ConfigError {
    ReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    ParseFailed {
        path: PathBuf,
        source: serde_json::Error,
    },
    MissingRoot {
        path: PathBuf,
    },
    UnmatchedRootGlob {
        pattern: String,
    },
    DuplicateModule {
        module: String,
    },
    EmptyRoots,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFailed { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Self::ParseFailed { path, source } => {
                write!(formatter, "failed to parse {}: {source}", path.display())
            }
            Self::MissingRoot { path } => {
                write!(
                    formatter,
                    "configured root does not exist: {}",
                    path.display()
                )
            }
            Self::UnmatchedRootGlob { pattern } => {
                write!(
                    formatter,
                    "configured root glob matched no paths: {pattern}"
                )
            }
            Self::DuplicateModule { module } => {
                write!(formatter, "duplicate configured module root: {module}")
            }
            Self::EmptyRoots => write!(formatter, "configuration must include at least one root"),
        }
    }
}

impl std::error::Error for ConfigError {}

pub fn load_project_config(path: &Path) -> Result<LoadedProjectConfig, ConfigError> {
    let config_path = if path.as_os_str().is_empty() {
        PathBuf::from("dead-code-finder.json")
    } else {
        path.to_path_buf()
    };
    let project_dir = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let raw = fs::read_to_string(&config_path).map_err(|source| ConfigError::ReadFailed {
        path: config_path.clone(),
        source,
    })?;
    let config: ProjectConfig =
        serde_json::from_str(&raw).map_err(|source| ConfigError::ParseFailed {
            path: config_path.clone(),
            source,
        })?;

    let roots = resolve_roots(&project_dir, &config)?;
    Ok(LoadedProjectConfig {
        config_path,
        project_dir,
        roots,
        entrypoints: config.entrypoints,
        include_tests: config.include_tests,
        test_patterns: config.test_patterns,
    })
}

pub fn resolve_roots(
    project_dir: &Path,
    config: &ProjectConfig,
) -> Result<Vec<ResolvedRoot>, ConfigError> {
    if config.roots.is_empty() {
        return Err(ConfigError::EmptyRoots);
    }

    let mut roots = Vec::new();
    for root in &config.roots {
        let paths = expand_root_path(project_dir, &root.path)?;
        for path in paths {
            let module = expand_module_template(&root.module, &path);
            roots.push(ResolvedRoot { path, module });
        }
    }

    roots.sort_by(|left, right| {
        left.module
            .cmp(&right.module)
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut modules = HashSet::new();
    for root in &roots {
        if !modules.insert(root.module.clone()) {
            return Err(ConfigError::DuplicateModule {
                module: root.module.clone(),
            });
        }
    }

    Ok(roots)
}

fn expand_root_path(
    project_dir: &Path,
    configured_path: &str,
) -> Result<Vec<PathBuf>, ConfigError> {
    if !configured_path.contains('*') {
        let path = project_dir.join(configured_path);
        if path.exists() {
            return Ok(vec![path]);
        }
        return Err(ConfigError::MissingRoot { path });
    }

    let parts = configured_path.split('/').collect::<Vec<_>>();
    let mut matches = Vec::new();
    expand_parts(project_dir.to_path_buf(), &parts, &mut matches);
    matches.sort();

    if matches.is_empty() {
        return Err(ConfigError::UnmatchedRootGlob {
            pattern: configured_path.to_string(),
        });
    }

    Ok(matches)
}

fn expand_parts(current: PathBuf, parts: &[&str], matches: &mut Vec<PathBuf>) {
    let Some((part, remaining)) = parts.split_first() else {
        if current.exists() {
            matches.push(current);
        }
        return;
    };

    if *part == "*" {
        let Ok(entries) = fs::read_dir(&current) else {
            return;
        };
        let mut children = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        children.sort();
        for child in children {
            expand_parts(child, remaining, matches);
        }
    } else {
        expand_parts(current.join(part), remaining, matches);
    }
}

fn expand_module_template(template: &str, path: &Path) -> String {
    let basename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    template.replace("{basename}", basename)
}

fn default_test_patterns() -> Vec<String> {
    vec![
        "test_*.py".to_string(),
        "*_test.py".to_string(),
        "tests/**".to_string(),
        "conftest.py".to_string(),
    ]
}

#[cfg(test)]
mod tests {
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
            include_tests: false,
            test_patterns: default_test_patterns(),
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
            include_tests: false,
            test_patterns: default_test_patterns(),
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
            include_tests: false,
            test_patterns: default_test_patterns(),
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
                "includeTests": true
            }"#,
        )
        .unwrap();

        let loaded = load_project_config(&workspace.join("dead-code-finder.json")).unwrap();

        assert_eq!(loaded.entrypoints, vec!["main.py"]);
        assert!(loaded.include_tests);
        assert_eq!(loaded.roots[0].module, "pkg");
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
}

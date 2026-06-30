use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[path = "config_rules.rs"]
mod config_rules;

use config_rules::validate_rules;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub roots: Vec<RootConfig>,
    #[serde(default)]
    pub type_sources: Vec<RootConfig>,
    #[serde(default)]
    pub root_groups: Vec<RootGroupConfig>,
    #[serde(default)]
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub weak_entrypoints: Vec<String>,
    #[serde(default)]
    pub include_tests: bool,
    #[serde(default = "default_test_patterns")]
    pub test_patterns: Vec<String>,
    #[serde(default)]
    pub rules: RuleConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootConfig {
    pub path: String,
    pub module: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootGroupConfig {
    pub name: String,
    #[serde(default)]
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub counts_as_used: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProjectConfig {
    pub config_path: PathBuf,
    pub project_dir: PathBuf,
    pub roots: Vec<ResolvedRoot>,
    pub type_sources: Vec<ResolvedRoot>,
    pub root_groups: Vec<LoadedRootGroup>,
    pub include_tests: bool,
    pub test_patterns: Vec<String>,
    pub rules: RuleConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRootGroup {
    pub name: String,
    pub entrypoints: Vec<String>,
    pub counts_as_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleConfig {
    #[serde(default)]
    pub constructors: Vec<ConstructorRule>,
    #[serde(default)]
    pub factory_returns: Vec<FactoryReturnRule>,
    #[serde(default)]
    pub class_surfaces: Vec<ClassSurfaceRule>,
    #[serde(default)]
    pub decorators: Vec<DecoratorRule>,
    #[serde(default)]
    pub calls: Vec<CallRule>,
    #[serde(default)]
    pub assignments: Vec<AssignmentRule>,
    #[serde(default)]
    pub fluent_methods: Vec<FluentMethodRule>,
    #[serde(default)]
    pub route_globs: Vec<RouteGlobRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstructorRule {
    #[serde(rename = "match")]
    pub match_: String,
    pub produces_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryReturnRule {
    pub function: String,
    #[serde(default)]
    pub type_keyword: String,
    #[serde(default)]
    pub type_position: Option<usize>,
    #[serde(default)]
    pub input_type_keyword: Option<String>,
    #[serde(default)]
    pub input_type_position: Option<usize>,
    #[serde(default)]
    pub return_container: Option<String>,
    #[serde(default)]
    pub mark_input_fields: bool,
    #[serde(default)]
    pub mark_output_fields: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassSurfaceRule {
    pub base: String,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecoratorRule {
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub receiver_type: Option<String>,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub callable_type: Option<String>,
    #[serde(default)]
    pub include_parameter_type_surface: bool,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallRule {
    #[serde(default)]
    pub function: Option<String>,
    #[serde(default)]
    pub receiver_type: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub member: Option<String>,
    #[serde(default)]
    pub target_argument: Option<usize>,
    #[serde(default)]
    pub member_argument: Option<usize>,
    pub effect: String,
    pub argument: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentRule {
    pub receiver_type: String,
    pub member: String,
    pub effect: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FluentMethodRule {
    pub receiver_type: String,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteGlobRule {
    pub when_function_called: String,
    pub glob: String,
    pub export: String,
    pub effect: String,
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
    DuplicateRootGroup {
        name: String,
    },
    EmptyRoots,
    EmptyRootGroupName,
    EmptyRootGroupEntrypoints {
        name: String,
    },
    InvalidRule {
        message: String,
    },
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
            Self::DuplicateRootGroup { name } => {
                write!(formatter, "duplicate configured root group: {name}")
            }
            Self::EmptyRoots => write!(formatter, "configuration must include at least one root"),
            Self::EmptyRootGroupName => write!(formatter, "root group name must not be empty"),
            Self::EmptyRootGroupEntrypoints { name } => {
                write!(
                    formatter,
                    "root group {name} must include at least one entrypoint"
                )
            }
            Self::InvalidRule { message } => write!(formatter, "invalid rule: {message}"),
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

    let roots = resolve_roots(&project_dir, &config.roots, true)?;
    let type_sources = resolve_roots(&project_dir, &config.type_sources, false)?;
    let root_groups = root_groups(&config)?;
    validate_rules(&config.rules)?;
    Ok(LoadedProjectConfig {
        config_path,
        project_dir,
        roots,
        type_sources,
        root_groups,
        include_tests: config.include_tests,
        test_patterns: config.test_patterns,
        rules: config.rules,
    })
}

fn root_groups(config: &ProjectConfig) -> Result<Vec<LoadedRootGroup>, ConfigError> {
    let mut groups = Vec::new();
    if config.root_groups.is_empty() {
        groups.push(LoadedRootGroup {
            name: "main".to_string(),
            entrypoints: config.entrypoints.clone(),
            counts_as_used: true,
        });
    } else {
        groups.extend(config.root_groups.iter().enumerate().map(|(index, group)| {
            LoadedRootGroup {
                name: group.name.clone(),
                entrypoints: group.entrypoints.clone(),
                counts_as_used: group.counts_as_used.unwrap_or(index == 0),
            }
        }));
    }
    if !config.weak_entrypoints.is_empty() {
        groups.push(LoadedRootGroup {
            name: "weak".to_string(),
            entrypoints: config.weak_entrypoints.clone(),
            counts_as_used: false,
        });
    }
    if config.include_tests {
        groups.push(LoadedRootGroup {
            name: "test".to_string(),
            entrypoints: config.test_patterns.clone(),
            counts_as_used: false,
        });
    }
    validate_root_groups(&groups)
}

fn validate_root_groups(groups: &[LoadedRootGroup]) -> Result<Vec<LoadedRootGroup>, ConfigError> {
    let mut names = HashSet::new();
    for group in groups {
        if group.name.trim().is_empty() {
            return Err(ConfigError::EmptyRootGroupName);
        }
        if !names.insert(group.name.clone()) {
            return Err(ConfigError::DuplicateRootGroup {
                name: group.name.clone(),
            });
        }
        if group.name != "main" && group.entrypoints.is_empty() {
            return Err(ConfigError::EmptyRootGroupEntrypoints {
                name: group.name.clone(),
            });
        }
    }
    Ok(groups.to_vec())
}

pub fn resolve_roots(
    project_dir: &Path,
    configured_roots: &[RootConfig],
    require_roots: bool,
) -> Result<Vec<ResolvedRoot>, ConfigError> {
    if require_roots && configured_roots.is_empty() {
        return Err(ConfigError::EmptyRoots);
    }

    let mut roots = Vec::new();
    for root in configured_roots {
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
        "*_test_*.py".to_string(),
        "tests/**".to_string(),
        "conftest.py".to_string(),
    ]
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;

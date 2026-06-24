//! Python analysis entrypoints.

use deadcode_core::AnalysisReport;

pub mod config;
pub mod reachability;
pub mod symbol_index;

use config::{load_project_config, ConfigError};
use reachability::{
    find_unused_symbols, unresolved_receiver_diagnostics, unsupported_expansion_diagnostics,
};
use symbol_index::{index_project, SymbolIndexError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub config_path: std::path::PathBuf,
}

impl AnalyzeOptions {
    pub fn new(config_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }
}

#[derive(Debug)]
pub enum AnalyzeError {
    Config(ConfigError),
    SymbolIndex(SymbolIndexError),
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "{error}"),
            Self::SymbolIndex(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AnalyzeError {}

pub fn analyze_project(options: &AnalyzeOptions) -> Result<AnalysisReport, AnalyzeError> {
    let config = load_project_config(&options.config_path).map_err(AnalyzeError::Config)?;
    let index = index_project(&config).map_err(AnalyzeError::SymbolIndex)?;
    let findings = find_unused_symbols(&index);
    let unresolved_diagnostics = unresolved_receiver_diagnostics(&index);
    let unsupported_expansion_diagnostics = unsupported_expansion_diagnostics(&index);
    let mut diagnostics = index
        .parse_diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.into_core_diagnostic())
        .collect::<Vec<_>>();
    diagnostics.extend(unresolved_diagnostics);
    diagnostics.extend(unsupported_expansion_diagnostics);
    Ok(AnalysisReport {
        findings,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_analysis_returns_clean_report() {
        let workspace = test_workspace("scaffold_analysis_returns_clean_report");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(workspace.join("pkg/__init__.py"), "").unwrap();
        std::fs::write(
            workspace.join("dead-code-finder.json"),
            r#"{"roots":[{"path":"pkg","module":"pkg"}]}"#,
        )
        .unwrap();

        let report = analyze_project(&AnalyzeOptions::new(
            workspace.join("dead-code-finder.json"),
        ))
        .unwrap();

        assert!(report.is_clean());
    }

    #[test]
    fn analysis_reports_unused_symbols() {
        let workspace = test_workspace("analysis_reports_unused_symbols");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(
            workspace.join("pkg/main.py"),
            r#"
def live():
    pass

def dead():
    pass

live()
"#,
        )
        .unwrap();
        std::fs::write(
            workspace.join("dead-code-finder.json"),
            r#"{
                "roots": [{"path": "pkg", "module": "pkg"}],
                "entrypoints": ["pkg/main.py"]
            }"#,
        )
        .unwrap();

        let report = analyze_project(&AnalyzeOptions::new(
            workspace.join("dead-code-finder.json"),
        ))
        .unwrap();

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].symbol, "pkg.main.dead");
    }

    fn test_workspace(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("deadcode_python_{name}_{unique}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}

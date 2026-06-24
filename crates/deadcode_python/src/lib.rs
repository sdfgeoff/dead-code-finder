//! Python analysis entrypoints.

use deadcode_core::AnalysisReport;

pub mod config;

use config::{load_project_config, ConfigError};

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
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for AnalyzeError {}

pub fn analyze_project(options: &AnalyzeOptions) -> Result<AnalysisReport, AnalyzeError> {
    let _config = load_project_config(&options.config_path).map_err(AnalyzeError::Config)?;
    Ok(AnalysisReport::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_analysis_returns_clean_report() {
        let workspace = test_workspace("scaffold_analysis_returns_clean_report");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
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

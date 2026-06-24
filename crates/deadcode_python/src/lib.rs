//! Python analysis entrypoints.
//!
//! This crate will own parsing, import resolution, symbol indexing, and
//! type-aware reachability. The current implementation is the ticket-001
//! scaffold used by the CLI and tests.

use std::path::{Path, PathBuf};

use deadcode_core::AnalysisReport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOptions {
    pub config_path: PathBuf,
}

impl AnalyzeOptions {
    pub fn new(config_path: impl Into<PathBuf>) -> Self {
        Self {
            config_path: config_path.into(),
        }
    }
}

pub fn analyze_project(options: &AnalyzeOptions) -> AnalysisReport {
    let _config_path = normalize_config_path(&options.config_path);
    AnalysisReport::default()
}

fn normalize_config_path(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        PathBuf::from("dead-code-finder.json")
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_analysis_returns_clean_report() {
        let report = analyze_project(&AnalyzeOptions::new("dead-code-finder.json"));

        assert!(report.is_clean());
    }
}

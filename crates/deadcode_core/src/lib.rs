//! Core report types shared by the analyzer and CLI.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Module,
    Function,
    Class,
    Method,
    Attribute,
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceSpan {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl SourceSpan {
    pub fn new(file: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub code: String,
    pub message: String,
    pub symbol: String,
    pub symbol_kind: SymbolKind,
    pub span: SourceSpan,
    pub reachable_from: Vec<String>,
}

impl Finding {
    pub fn unused(
        code: impl Into<String>,
        symbol: impl Into<String>,
        symbol_kind: SymbolKind,
        span: SourceSpan,
    ) -> Self {
        let symbol = symbol.into();
        let mut finding = Self {
            code: code.into(),
            message: format!("unused symbol {symbol}"),
            symbol,
            symbol_kind,
            span,
            reachable_from: Vec::new(),
        };
        finding.refresh_message();
        finding
    }

    pub fn with_reachable_from(mut self, root_sets: Vec<String>) -> Self {
        self.reachable_from = root_sets;
        self.refresh_message();
        self
    }

    fn refresh_message(&mut self) {
        self.message = if self.reachable_from.is_empty() {
            format!("unused symbol {}", self.symbol)
        } else {
            format!(
                "unused symbol {} (reachable from {} only)",
                self.symbol,
                self.reachable_from.join(", ")
            )
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub span: SourceSpan,
}

impl Diagnostic {
    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct AnalysisReport {
    pub findings: Vec<Finding>,
    pub diagnostics: Vec<Diagnostic>,
}

impl AnalysisReport {
    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn summary(&self) -> ReportSummary {
        ReportSummary {
            findings: self.findings.len(),
            diagnostics: self.diagnostics.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReportSummary {
    pub findings: usize,
    pub diagnostics: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_report_has_no_findings() {
        let report = AnalysisReport::default();

        assert!(report.is_clean());
        assert_eq!(
            report.summary(),
            ReportSummary {
                findings: 0,
                diagnostics: 0
            }
        );
    }

    #[test]
    fn findings_make_report_unclean() {
        let report = AnalysisReport {
            findings: vec![Finding::unused(
                "DCF001",
                "example.dead",
                SymbolKind::Function,
                SourceSpan::new("example.py", 1, 1),
            )],
            diagnostics: vec![],
        };

        assert!(!report.is_clean());
        assert_eq!(report.summary().findings, 1);
        assert!(report.findings[0].reachable_from.is_empty());
    }
}

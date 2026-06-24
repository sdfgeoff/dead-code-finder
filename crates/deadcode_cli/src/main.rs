use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use deadcode_core::{AnalysisReport, Diagnostic, Finding, ReportSummary, Severity};
use deadcode_python::{analyze_project, AnalyzeOptions};
use serde::Serialize;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let options = match parse_options(&args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    let report = match analyze_project(&AnalyzeOptions::new(options.config_path)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("dead-code-finder: {error}");
            return ExitCode::from(2);
        }
    };
    let summary = report.summary();

    match options.format {
        OutputFormat::Text => print_text_report(&report, summary),
        OutputFormat::Json => print_json_report(&report, summary),
    }

    if report.is_clean() && !(options.strict && !report.diagnostics.is_empty()) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    config_path: PathBuf,
    format: OutputFormat,
    strict: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

fn parse_options(args: &[String]) -> Result<CliOptions, String> {
    let mut options = CliOptions {
        config_path: PathBuf::from("dead-code-finder.json"),
        format: OutputFormat::Text,
        strict: false,
    };
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--config" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("--config requires a path".to_string());
                };
                options.config_path = PathBuf::from(path);
                index += 2;
            }
            "--format" => {
                let Some(format) = args.get(index + 1) else {
                    return Err("--format requires text or json".to_string());
                };
                options.format = match format.as_str() {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err("--format requires text or json".to_string()),
                };
                index += 2;
            }
            "--strict" => {
                options.strict = true;
                index += 1;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    Ok(options)
}

fn print_usage() {
    eprintln!(
        "usage: dead-code-finder [--config dead-code-finder.json] [--format text|json] [--strict]"
    );
}

fn print_text_report(report: &AnalysisReport, summary: ReportSummary) {
    for diagnostic in &report.diagnostics {
        print_diagnostic(diagnostic);
    }
    for finding in &report.findings {
        print_finding(finding);
    }

    println!(
        "dead-code-finder: {} finding(s), {} diagnostic(s)",
        summary.findings, summary.diagnostics
    );
}

#[derive(Serialize)]
struct JsonReport<'a> {
    findings: &'a [Finding],
    diagnostics: &'a [Diagnostic],
    summary: ReportSummary,
}

fn print_json_report(report: &AnalysisReport, summary: ReportSummary) {
    let json = JsonReport {
        findings: &report.findings,
        diagnostics: &report.diagnostics,
        summary,
    };
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

fn print_finding(finding: &Finding) {
    println!(
        "{}:{}:{} {} {}",
        finding.span.file, finding.span.line, finding.span.column, finding.code, finding.message
    );
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    println!(
        "{}:{}:{} {} {}: {}",
        diagnostic.span.file,
        diagnostic.span.line,
        diagnostic.span.column,
        diagnostic.code,
        severity_label(&diagnostic.severity),
        diagnostic.message
    );
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

use std::env;
use std::io::{self, ErrorKind, Write};
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

    let print_result = match options.format {
        OutputFormat::Text => print_text_report(&report, summary),
        OutputFormat::Json => print_json_report(&report, summary),
    };
    if let Err(error) = print_result {
        if error.kind() == ErrorKind::BrokenPipe {
            return ExitCode::SUCCESS;
        }
        eprintln!("dead-code-finder: failed to write output: {error}");
        return ExitCode::from(2);
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

fn print_text_report(report: &AnalysisReport, summary: ReportSummary) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    for diagnostic in &report.diagnostics {
        print_diagnostic(&mut writer, diagnostic)?;
    }
    for finding in &report.findings {
        print_finding(&mut writer, finding)?;
    }

    writeln!(
        writer,
        "dead-code-finder: {} finding(s), {} diagnostic(s)",
        summary.findings, summary.diagnostics
    )
}

#[derive(Serialize)]
struct JsonReport<'a> {
    findings: &'a [Finding],
    diagnostics: &'a [Diagnostic],
    summary: ReportSummary,
}

fn print_json_report(report: &AnalysisReport, summary: ReportSummary) -> io::Result<()> {
    let json = JsonReport {
        findings: &report.findings,
        diagnostics: &report.diagnostics,
        summary,
    };
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    serde_json::to_writer_pretty(&mut writer, &json).unwrap();
    writeln!(writer)
}

fn print_finding(writer: &mut impl Write, finding: &Finding) -> io::Result<()> {
    writeln!(
        writer,
        "{}:{}:{} {} {}",
        finding.span.file, finding.span.line, finding.span.column, finding.code, finding.message
    )
}

fn print_diagnostic(writer: &mut impl Write, diagnostic: &Diagnostic) -> io::Result<()> {
    writeln!(
        writer,
        "{}:{}:{} {} {}: {}",
        diagnostic.span.file,
        diagnostic.span.line,
        diagnostic.span.column,
        diagnostic.code,
        severity_label(&diagnostic.severity),
        diagnostic.message
    )
}

fn severity_label(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}

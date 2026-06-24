use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use deadcode_core::{Diagnostic, Finding, Severity};
use deadcode_python::{analyze_project, AnalyzeOptions};

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let config_path = match parse_config_path(&args) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{message}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    let report = match analyze_project(&AnalyzeOptions::new(config_path)) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("dead-code-finder: {error}");
            return ExitCode::from(2);
        }
    };
    let summary = report.summary();

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

    if report.is_clean() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn parse_config_path(args: &[String]) -> Result<PathBuf, String> {
    let mut config_path = PathBuf::from("dead-code-finder.json");
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--config" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("--config requires a path".to_string());
                };
                config_path = PathBuf::from(path);
                index += 2;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    Ok(config_path)
}

fn print_usage() {
    eprintln!("usage: dead-code-finder [--config dead-code-finder.json]");
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

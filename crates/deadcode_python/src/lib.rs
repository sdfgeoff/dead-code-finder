//! Python analysis entrypoints.

use deadcode_core::AnalysisReport;

pub mod config;
pub mod reachability;
mod symbol_files;
pub mod symbol_index;
mod symbol_roots;

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

    #[test]
    fn declarative_rules_register_decorated_functions() {
        let workspace = test_workspace("declarative_rules_register_decorated_functions");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(
            workspace.join("pkg/main.py"),
            r#"
from toyframework import Router

router = Router()

@router.get("/items")
def list_items():
    pass
"#,
        )
        .unwrap();
        std::fs::write(
            workspace.join("dead-code-finder.json"),
            r#"{
                "roots": [{"path": "pkg", "module": "pkg"}],
                "entrypoints": ["pkg/main.py"],
                "rules": {
                    "constructors": [{
                        "match": "toyframework.Router",
                        "producesType": "toyframework.Router"
                    }],
                    "decorators": [{
                        "receiverType": "toyframework.Router",
                        "methods": ["get"],
                        "effect": "registerDecoratedFunction"
                    }]
                }
            }"#,
        )
        .unwrap();

        let report = analyze_project(&AnalyzeOptions::new(
            workspace.join("dead-code-finder.json"),
        ))
        .unwrap();

        assert!(report.is_clean());
    }

    #[test]
    fn decorators_without_rules_do_not_keep_functions_alive() {
        let workspace = test_workspace("decorators_without_rules_do_not_keep_functions_alive");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(
            workspace.join("pkg/main.py"),
            r#"
from toyframework import Router

router = Router()

@router.get("/items")
def list_items():
    pass
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
        assert_eq!(report.findings[0].symbol, "pkg.main.list_items");
    }

    #[test]
    fn fastapi_rules_keep_routes_and_dependencies_alive() {
        let workspace = test_workspace("fastapi_rules_keep_routes_and_dependencies_alive");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(
            workspace.join("pkg/main.py"),
            r#"
from fastapi import Depends, FastAPI

app = FastAPI()

def get_user():
    pass

def unused_dependency():
    pass

@app.get("/items")
def list_items(entity = Depends(get_user)):
    pass
"#,
        )
        .unwrap();
        std::fs::write(
            workspace.join("dead-code-finder.json"),
            r#"{
                "roots": [{"path": "pkg", "module": "pkg"}],
                "entrypoints": ["pkg/main.py"],
                "rules": {
                    "constructors": [{
                        "match": "fastapi.FastAPI",
                        "producesType": "fastapi.FastAPI"
                    }],
                    "decorators": [{
                        "receiverType": "fastapi.FastAPI",
                        "methods": ["get"],
                        "effect": "registerDecoratedFunction"
                    }],
                    "calls": [{
                        "function": "fastapi.Depends",
                        "effect": "useCallableArgument",
                        "argument": 0
                    }]
                }
            }"#,
        )
        .unwrap();

        let report = analyze_project(&AnalyzeOptions::new(
            workspace.join("dead-code-finder.json"),
        ))
        .unwrap();

        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].symbol, "pkg.main.unused_dependency");
    }

    #[test]
    fn fastapi_rules_cover_router_decorators_and_include_router_config() {
        let workspace =
            test_workspace("fastapi_rules_cover_router_decorators_and_include_router_config");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(
            workspace.join("pkg/main.py"),
            r#"
from fastapi import APIRouter, FastAPI

app = FastAPI()
router = APIRouter()

@router.get("/items")
def list_items():
    pass

app.include_router(router)
"#,
        )
        .unwrap();
        std::fs::write(
            workspace.join("dead-code-finder.json"),
            r#"{
                "roots": [{"path": "pkg", "module": "pkg"}],
                "entrypoints": ["pkg/main.py"],
                "rules": {
                    "constructors": [
                        {
                            "match": "fastapi.FastAPI",
                            "producesType": "fastapi.FastAPI"
                        },
                        {
                            "match": "fastapi.APIRouter",
                            "producesType": "fastapi.APIRouter"
                        }
                    ],
                    "decorators": [{
                        "receiverType": "fastapi.APIRouter",
                        "methods": ["get"],
                        "effect": "registerDecoratedFunction"
                    }],
                    "calls": [{
                        "receiverType": "fastapi.FastAPI",
                        "method": "include_router",
                        "effect": "connectRouter",
                        "argument": 0
                    }]
                }
            }"#,
        )
        .unwrap();

        let report = analyze_project(&AnalyzeOptions::new(
            workspace.join("dead-code-finder.json"),
        ))
        .unwrap();

        assert!(report.is_clean());
    }

    #[test]
    fn pydantic_style_construction_uses_only_explicit_fields() {
        let workspace = test_workspace("pydantic_style_construction_uses_only_explicit_fields");
        std::fs::create_dir_all(workspace.join("pkg")).unwrap();
        std::fs::write(
            workspace.join("pkg/main.py"),
            r#"
from pydantic import BaseModel

class ExampleEntity(BaseModel):
    name: str
    age: int

def run():
    ExampleEntity(name="A")

run()
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
        let symbols = report
            .findings
            .iter()
            .map(|finding| finding.symbol.as_str())
            .collect::<Vec<_>>();

        assert!(!symbols.contains(&"pkg.main.ExampleEntity.name"));
        assert!(symbols.contains(&"pkg.main.ExampleEntity.age"));
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

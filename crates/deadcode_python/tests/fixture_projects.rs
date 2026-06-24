use std::fs;
use std::path::PathBuf;

use deadcode_python::{analyze_project, AnalyzeOptions};

#[test]
fn multi_root_workspace_resolves_cross_root_usage() {
    let project = FixtureProject::new("multi_root_workspace_resolves_cross_root_usage");
    project.write(
        "apps/api/main.py",
        "from shared.service import live\n\nlive()\n",
    );
    project.write(
        "libs/shared/service.py",
        "def live():\n    pass\n\ndef dead():\n    pass\n",
    );
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [
                {"path": "apps/api", "module": "api"},
                {"path": "libs/shared", "module": "shared"}
            ],
            "entrypoints": ["apps/api/main.py"]
        }"#,
    );

    let report = project.analyze();
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"shared.service.live".to_string()));
    assert!(symbols.contains(&"shared.service.dead".to_string()));
}

#[test]
fn package_glob_roots_expand_for_workspace_packages() {
    let project = FixtureProject::new("package_glob_roots_expand_for_workspace_packages");
    project.write(
        "packages/one/src/one/app.py",
        "from two.lib import live\n\nlive()\n",
    );
    project.write(
        "packages/two/src/two/lib.py",
        "def live():\n    pass\n\ndef dead():\n    pass\n",
    );
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "packages/*/src/*", "module": "{basename}"}],
            "entrypoints": ["packages/one/src/one/app.py"]
        }"#,
    );

    let report = project.analyze();
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"two.lib.live".to_string()));
    assert!(symbols.contains(&"two.lib.dead".to_string()));
}

#[test]
fn fastapi_style_rules_cover_routes_dependencies_and_models() {
    let project = FixtureProject::new("fastapi_style_rules_cover_routes_dependencies_and_models");
    project.write(
        "api/main.py",
        r#"
from fastapi import APIRouter, Depends, FastAPI
from pydantic import BaseModel

app = FastAPI()
router = APIRouter()

class ExampleEntity(BaseModel):
    name: str
    age: int

def get_user():
    return ExampleEntity(name="Ada")

def unused_dependency():
    pass

@router.get("/entities")
def list_users(entity = Depends(get_user)):
    pass

app.include_router(router)
"#,
    );
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "api", "module": "api"}],
            "entrypoints": ["api/main.py"],
            "rules": {
                "constructors": [
                    {"match": "fastapi.FastAPI", "producesType": "fastapi.FastAPI"},
                    {"match": "fastapi.APIRouter", "producesType": "fastapi.APIRouter"}
                ],
                "decorators": [{
                    "receiverType": "fastapi.APIRouter",
                    "methods": ["get"],
                    "effect": "registerDecoratedFunction"
                }],
                "calls": [
                    {"function": "fastapi.Depends", "effect": "useCallableArgument", "argument": 0},
                    {
                        "receiverType": "fastapi.FastAPI",
                        "method": "include_router",
                        "effect": "connectRouter",
                        "argument": 0
                    }
                ]
            }
        }"#,
    );

    let report = project.analyze();
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"api.main.list_users".to_string()));
    assert!(!symbols.contains(&"api.main.get_user".to_string()));
    assert!(!symbols.contains(&"api.main.ExampleEntity.name".to_string()));
    assert!(symbols.contains(&"api.main.ExampleEntity.age".to_string()));
    assert!(symbols.contains(&"api.main.unused_dependency".to_string()));
}

#[test]
fn scripts_inheritance_generics_and_unresolved_receivers_are_reported() {
    let project = FixtureProject::new("scripts_inheritance_generics_and_unresolved_receivers");
    project.write(
        "pkg/script.py",
        r#"
class ExampleEntity:
    def save(self):
        pass

class Other:
    def save(self):
        pass

class Box[T]:
    value: T

def process(box: Box[ExampleEntity]):
    entity = box.value
    entity.save()

def unresolved(x):
    x.save()

if __name__ == "__main__":
    process(Box())
    unresolved(None)
"#,
    );
    project.write(
        "dead-code-finder.json",
        r#"{"roots": [{"path": "pkg", "module": "pkg"}]}"#,
    );

    let report = project.analyze();
    let symbols = finding_symbols(&report);
    let diagnostics = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert!(!symbols.contains(&"pkg.script.ExampleEntity.save".to_string()));
    assert!(symbols.contains(&"pkg.script.Other.save".to_string()));
    assert!(diagnostics.contains(&"DCF101"));
}

#[test]
fn generated_service_graph_is_deterministic_and_debuggable() {
    let project = FixtureProject::new("generated_service_graph_is_deterministic_and_debuggable");
    let mut source = String::new();
    source.push_str("def entry():\n    node_0()\n\n");
    for index in 0..6 {
        source.push_str(&format!("def node_{index}():\n"));
        if index < 3 {
            source.push_str(&format!("    node_{}()\n\n", index + 1));
        } else {
            source.push_str("    pass\n\n");
        }
    }
    source.push_str("entry()\n");
    project.write("pkg/generated.py", &source);
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/generated.py"]
        }"#,
    );

    let report = project.analyze();
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"pkg.generated.node_3".to_string()));
    assert!(symbols.contains(&"pkg.generated.node_4".to_string()));
    assert!(symbols.contains(&"pkg.generated.node_5".to_string()));
}

#[test]
fn test_usage_does_not_keep_production_symbols_alive() {
    let project = FixtureProject::new("test_usage_does_not_keep_production_symbols_alive");
    project.write("pkg/service.py", "def helper():\n    pass\n");
    project.write("pkg/main.py", "def entry():\n    pass\n\nentry()\n");
    project.write(
        "pkg/tests/test_service.py",
        "from pkg.service import helper\n\ndef test_helper():\n    helper()\n",
    );
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/main.py"],
            "includeTests": true
        }"#,
    );

    let report = project.analyze();
    let helper = report
        .findings
        .iter()
        .find(|finding| finding.symbol == "pkg.service.helper")
        .unwrap();

    assert_eq!(helper.reachable_from, vec!["test".to_string()]);
}

#[test]
fn include_tests_reports_dead_helpers_inside_test_files() {
    let project = FixtureProject::new("include_tests_reports_dead_helpers_inside_test_files");
    project.write("pkg/main.py", "def entry():\n    pass\n\nentry()\n");
    project.write(
        "pkg/tests/test_service.py",
        "def test_live():\n    pass\n\ndef dead_helper():\n    pass\n",
    );
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/main.py"],
            "includeTests": true
        }"#,
    );

    let report = project.analyze();
    let symbols = finding_symbols(&report);

    assert!(!symbols.contains(&"pkg.tests.test_service.test_live".to_string()));
    assert!(symbols.contains(&"pkg.tests.test_service.dead_helper".to_string()));
}

struct FixtureProject {
    root: PathBuf,
}

impl FixtureProject {
    fn new(name: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("deadcode_fixture_{name}_{unique}"));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn analyze(&self) -> deadcode_core::AnalysisReport {
        analyze_project(&AnalyzeOptions::new(
            self.root.join("dead-code-finder.json"),
        ))
        .unwrap_or_else(|error| panic!("fixture at {} failed: {error}", self.root.display()))
    }
}

fn finding_symbols(report: &deadcode_core::AnalysisReport) -> Vec<String> {
    report
        .findings
        .iter()
        .map(|finding| finding.symbol.clone())
        .collect()
}

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn text_output_reports_findings_and_exits_one() {
    let project = FixtureProject::new("text_output_reports_findings_and_exits_one");
    project.write("pkg/main.py", "def dead():\n    pass\n");
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/main.py"]
        }"#,
    );

    let output = project.command().output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("DCF001 unused symbol pkg.main.dead"));
    assert!(stdout.contains("dead-code-finder: 1 finding(s), 0 diagnostic(s)"));
}

#[test]
fn json_output_contains_findings_diagnostics_and_summary() {
    let project = FixtureProject::new("json_output_contains_findings_diagnostics_and_summary");
    project.write("pkg/main.py", "def dead():\n    pass\n");
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/main.py"]
        }"#,
    );

    let output = project
        .command()
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["findings"], 1);
    assert_eq!(json["summary"]["diagnostics"], 0);
    assert_eq!(json["findings"][0]["code"], "DCF001");
    assert_eq!(json["findings"][0]["symbol"], "pkg.main.dead");
    assert_eq!(json["findings"][0]["span"]["line"], 1);
}

#[test]
fn unresolved_diagnostics_warn_by_default_and_fail_in_strict_mode() {
    let project = FixtureProject::new("unresolved_diagnostics_warn_by_default");
    project.write(
        "pkg/main.py",
        r#"
def run(x):
    x.save()

run(None)
"#,
    );
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/main.py"]
        }"#,
    );

    let default_output = project.command().output().unwrap();
    assert_eq!(default_output.status.code(), Some(0));
    let stdout = String::from_utf8(default_output.stdout).unwrap();
    assert!(stdout.contains("DCF101 warning: cannot resolve receiver type for x.save"));

    let strict_output = project.command().arg("--strict").output().unwrap();
    assert_eq!(strict_output.status.code(), Some(1));
}

#[test]
#[cfg(unix)]
fn broken_pipe_exits_like_a_unix_filter() {
    let project = FixtureProject::new("broken_pipe_exits_like_a_unix_filter");
    let mut source = String::new();
    for index in 0..20_000 {
        source.push_str(&format!("def dead_{index}():\n    pass\n\n"));
    }
    project.write("pkg/main.py", &source);
    project.write(
        "dead-code-finder.json",
        r#"{
            "roots": [{"path": "pkg", "module": "pkg"}],
            "entrypoints": ["pkg/main.py"]
        }"#,
    );

    let mut producer = project.command();
    let mut producer = producer.stdout(Stdio::piped()).spawn().unwrap();
    let stdout = producer.stdout.take().unwrap();
    let head = Command::new("head")
        .args(["-n", "1"])
        .stdin(stdout)
        .stdout(Stdio::null())
        .status()
        .unwrap();
    let producer = producer.wait().unwrap();

    assert!(head.success());
    assert_eq!(producer.code(), Some(141));
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
        let root = std::env::temp_dir().join(format!("deadcode_cli_{name}_{unique}"));
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

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dead-code-finder"));
        command.args([
            "--config",
            self.root.join("dead-code-finder.json").to_str().unwrap(),
        ]);
        command
    }
}

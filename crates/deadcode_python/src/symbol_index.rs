#[path = "symbol_collector.rs"]
mod symbol_collector;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use deadcode_core::{Diagnostic, Severity, SourceSpan, SymbolKind};
use ruff_text_size::TextRange;

use crate::config::{LoadedProjectConfig, ResolvedRoot, RuleConfig};

use self::symbol_collector::SymbolCollector;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolIndex {
    pub modules: Vec<ModuleIndex>,
    pub parse_diagnostics: Vec<ParseDiagnostic>,
    pub include_tests: bool,
    pub route_globs: Vec<ResolvedRouteGlob>,
    known_modules: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRouteGlob {
    pub when_function_called: String,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleIndex {
    pub module: String,
    pub file: PathBuf,
    pub symbols: Vec<IndexedSymbol>,
    pub imports: Vec<ResolvedImport>,
    pub classes: Vec<ClassInfo>,
    pub function_signatures: Vec<FunctionSignature>,
    pub call_argument_types: Vec<CallArgumentType>,
    pub references: Vec<SymbolReference>,
    pub member_references: Vec<MemberReference>,
    pub unresolved_receivers: Vec<UnresolvedReceiver>,
    pub unsupported_expansions: Vec<UnsupportedExpansion>,
    pub is_entrypoint: bool,
    pub is_test: bool,
    pub test_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub qualified_name: String,
    pub name: String,
    pub kind: SymbolKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassInfo {
    pub class: String,
    pub bases: Vec<String>,
    pub type_params: Vec<String>,
    pub fields: Vec<ClassFieldInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassFieldInfo {
    pub name: String,
    pub annotation: FieldAnnotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAnnotation {
    Concrete(String),
    TypeParam(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub function: String,
    pub parameter_types: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallArgumentType {
    pub from: String,
    pub callee: String,
    pub position: usize,
    pub concrete_type: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedImport {
    pub binding: String,
    pub target: ImportTarget,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    Module {
        module: String,
        external: bool,
    },
    Symbol {
        module: String,
        name: String,
        external: bool,
    },
    Star {
        module: String,
        external: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolReference {
    pub from: String,
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberReference {
    pub from: String,
    pub target: String,
    pub access: AccessKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
    Construct,
    Call,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReceiver {
    pub from: String,
    pub receiver: String,
    pub member: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedExpansion {
    pub from: String,
    pub target: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub file: PathBuf,
    pub message: String,
    pub span: SourceSpan,
}

impl ParseDiagnostic {
    pub fn into_core_diagnostic(self) -> Diagnostic {
        Diagnostic {
            code: "DCF102".to_string(),
            severity: Severity::Warning,
            message: self.message,
            span: self.span,
        }
    }
}

#[derive(Debug)]
pub enum SymbolIndexError {
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    ReadDirectory {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for SymbolIndexError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFile { path, source } => {
                write!(
                    formatter,
                    "failed to read Python file {}: {source}",
                    path.display()
                )
            }
            Self::ReadDirectory { path, source } => {
                write!(
                    formatter,
                    "failed to read directory {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SymbolIndexError {}

pub fn index_project(config: &LoadedProjectConfig) -> Result<SymbolIndex, SymbolIndexError> {
    let mut index = SymbolIndex::default();
    index.include_tests = config.include_tests;

    for root in &config.roots {
        let mut files = Vec::new();
        collect_python_files(&root.path, &mut files)?;
        files.sort();

        for file in files {
            let module = module_name_for_file(root, &file);
            index.known_modules.insert(module);
        }
    }

    for root in &config.roots {
        let mut files = Vec::new();
        collect_python_files(&root.path, &mut files)?;
        files.sort();

        for file in files {
            let module = module_name_for_file(root, &file);
            let module_index = index_module(
                &module,
                &file,
                &index.known_modules,
                &config.rules,
                is_configured_entrypoint(config, &file),
                is_test_file(config, &file),
            )?;
            index
                .parse_diagnostics
                .extend(module_index.parse_diagnostics);
            index.modules.push(module_index.module);
        }
    }

    index.modules.sort_by(|left, right| {
        left.module
            .cmp(&right.module)
            .then_with(|| left.file.cmp(&right.file))
    });
    index.route_globs = resolve_route_globs(config, &index.modules);

    index.known_modules.clear();

    Ok(index)
}

fn resolve_route_globs(
    config: &LoadedProjectConfig,
    modules: &[ModuleIndex],
) -> Vec<ResolvedRouteGlob> {
    config
        .rules
        .route_globs
        .iter()
        .map(|rule| {
            let modules = modules
                .iter()
                .filter(|module| route_glob_matches(&config.project_dir, &rule.glob, &module.file))
                .map(|module| module.module.clone())
                .collect();
            ResolvedRouteGlob {
                when_function_called: rule.when_function_called.clone(),
                modules,
            }
        })
        .collect()
}

fn route_glob_matches(project_dir: &Path, pattern: &str, file: &Path) -> bool {
    let relative = file.strip_prefix(project_dir).unwrap_or(file);
    let relative = relative.to_string_lossy().replace('\\', "/");
    if let Some((prefix, suffix)) = pattern.split_once("**") {
        return relative.starts_with(prefix) && relative.ends_with(suffix.trim_start_matches('/'));
    }
    relative == pattern
}

fn collect_python_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), SymbolIndexError> {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "py") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|source| SymbolIndexError::ReadDirectory {
        path: path.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| SymbolIndexError::ReadDirectory {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_python_files(&entry_path, files)?;
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "py")
        {
            files.push(entry_path);
        }
    }

    Ok(())
}

struct IndexedModuleResult {
    module: ModuleIndex,
    parse_diagnostics: Vec<ParseDiagnostic>,
}

fn index_module(
    module: &str,
    file: &Path,
    known_modules: &HashSet<String>,
    rules: &RuleConfig,
    is_configured_entrypoint: bool,
    is_test: bool,
) -> Result<IndexedModuleResult, SymbolIndexError> {
    let source = fs::read_to_string(file).map_err(|source| SymbolIndexError::ReadFile {
        path: file.to_path_buf(),
        source,
    })?;
    let locator = SourceLocator::new(&source);
    let file_display = file.display().to_string();
    let mut symbols = vec![IndexedSymbol {
        qualified_name: module.to_string(),
        name: module.to_string(),
        kind: SymbolKind::Module,
        span: SourceSpan::new(file_display.clone(), 1, 1),
    }];
    let mut imports = Vec::new();
    let mut classes = Vec::new();
    let mut function_signatures = Vec::new();
    let mut call_argument_types = Vec::new();
    let mut references = Vec::new();
    let mut member_references = Vec::new();
    let mut unresolved_receivers = Vec::new();
    let mut unsupported_expansions = Vec::new();
    let mut has_main_entrypoint = false;
    let mut parse_diagnostics = Vec::new();

    match ruff_python_parser::parse_module(&source) {
        Ok(parsed) => {
            let suite = parsed.suite();
            let mut collector = SymbolCollector {
                module,
                file: &file_display,
                locator: &locator,
                symbols: &mut symbols,
                imports: &mut imports,
                classes: &mut classes,
                fn_sigs: &mut function_signatures,
                call_args: &mut call_argument_types,
                references: &mut references,
                member_refs: &mut member_references,
                unresolved_receivers: &mut unresolved_receivers,
                unsupported: &mut unsupported_expansions,
                main_entry: &mut has_main_entrypoint,
                known_modules,
                rules,
            };
            collector.collect_suite(suite);
        }
        Err(error) => {
            let span = locator.span(file, error.location);
            parse_diagnostics.push(ParseDiagnostic {
                file: file.to_path_buf(),
                message: format!("could not parse Python module: {}", error.error),
                span,
            });
        }
    }

    let test_roots = if is_test {
        symbols
            .iter()
            .filter(|symbol| {
                symbol.kind == SymbolKind::Function && symbol.name.starts_with("test_")
            })
            .map(|symbol| symbol.qualified_name.clone())
            .collect()
    } else {
        Vec::new()
    };

    Ok(IndexedModuleResult {
        module: ModuleIndex {
            module: module.to_string(),
            file: file.to_path_buf(),
            symbols,
            imports,
            classes,
            function_signatures,
            call_argument_types,
            references,
            member_references,
            unresolved_receivers,
            unsupported_expansions,
            is_entrypoint: is_configured_entrypoint || has_main_entrypoint,
            is_test,
            test_roots,
        },
        parse_diagnostics,
    })
}

fn module_name_for_file(root: &ResolvedRoot, file: &Path) -> String {
    let relative = file.strip_prefix(&root.path).unwrap_or(file);
    let mut parts = relative
        .iter()
        .filter_map(|part| part.to_str())
        .map(strip_py_extension)
        .filter(|part| part != "__init__")
        .collect::<Vec<_>>();
    parts.insert(0, root.module.clone());
    parts.join(".")
}

fn is_configured_entrypoint(config: &LoadedProjectConfig, file: &Path) -> bool {
    config.entrypoints.iter().any(|entrypoint| {
        let configured = config.project_dir.join(entrypoint);
        configured == file
    })
}

fn is_test_file(config: &LoadedProjectConfig, file: &Path) -> bool {
    let relative = file.strip_prefix(&config.project_dir).unwrap_or(file);
    let relative_text = relative.to_string_lossy();
    let filename = file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    config
        .test_patterns
        .iter()
        .any(|pattern| match pattern.as_str() {
            "tests/**" => relative
                .components()
                .any(|part| part.as_os_str() == "tests"),
            "test_*.py" => filename.starts_with("test_") && filename.ends_with(".py"),
            "*_test.py" => filename.ends_with("_test.py"),
            "conftest.py" => filename == "conftest.py",
            pattern => relative_text == pattern,
        })
}

fn strip_py_extension(part: &str) -> String {
    part.strip_suffix(".py").unwrap_or(part).to_string()
}

struct SourceLocator {
    line_starts: Vec<usize>,
}

impl SourceLocator {
    fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn span(&self, file: &Path, range: TextRange) -> SourceSpan {
        self.span_from_range_string(&file.display().to_string(), range)
    }

    fn span_from_range_string(&self, file: &str, range: TextRange) -> SourceSpan {
        let offset = range.start().to_usize();
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        SourceSpan::new(
            file,
            line_index + 1,
            offset - self.line_starts[line_index] + 1,
        )
    }
}

#[cfg(test)]
#[path = "symbol_index_tests.rs"]
mod symbol_index_tests;

#[path = "symbol_collector.rs"]
mod symbol_collector;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use deadcode_core::{Diagnostic, Severity, SourceSpan, SymbolKind};
use ruff_text_size::TextRange;

use crate::config::{LoadedProjectConfig, ResolvedRoot, RuleConfig};

use self::symbol_collector::SymbolCollector;

pub(crate) type ReexportMap = HashMap<(String, String), ImportTarget>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolIndex {
    pub modules: Vec<ModuleIndex>,
    pub parse_diagnostics: Vec<ParseDiagnostic>,
    pub include_tests: bool,
    pub include_weak: bool,
    pub class_surfaces: Vec<String>,
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
    pub value_bindings: Vec<ValueBinding>,
    pub function_signatures: Vec<FunctionSignature>,
    pub call_argument_types: Vec<CallArgumentType>,
    pub references: Vec<SymbolReference>,
    pub member_references: Vec<MemberReference>,
    pub unresolved_receivers: Vec<UnresolvedReceiver>,
    pub unsupported_expansions: Vec<UnsupportedExpansion>,
    pub is_entrypoint: bool,
    pub is_weak_entrypoint: bool,
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
    pub bases: Vec<TypeBinding>,
    pub type_params: Vec<String>,
    pub fields: Vec<ClassFieldInfo>,
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassFieldInfo {
    pub name: String,
    pub annotation: FieldAnnotation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeBinding {
    pub base: String,
    pub args: Vec<TypeBinding>,
    pub external: bool,
}

impl TypeBinding {
    pub fn erased(base: String) -> Self {
        Self {
            base,
            args: Vec::new(),
            external: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAnnotation {
    Concrete(TypeBinding),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueBinding {
    pub qualified_name: String,
    pub binding: TypeBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    pub function: String,
    pub parameters: Vec<FunctionParameter>,
    pub return_type: Option<TypeBinding>,
    pub concrete_return_type: Option<TypeBinding>,
    pub validated_return_types: Vec<TypeBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParameter {
    pub name: String,
    pub annotation: Option<TypeBinding>,
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
    index.include_weak = !config.weak_entrypoints.is_empty();
    index.class_surfaces = config
        .rules
        .class_surfaces
        .iter()
        .map(|rule| rule.base.clone())
        .collect();
    let mut project_files = Vec::new();

    for root in &config.roots {
        let mut files = Vec::new();
        collect_python_files(&root.path, &mut files)?;
        files.sort();

        for file in files {
            let module = module_name_for_file(root, &file);
            index.known_modules.insert(module.clone());
            project_files.push((file, module));
        }
    }

    for (file, module) in &project_files {
        let module_index = index_module(
            module,
            file,
            &index.known_modules,
            &config.rules,
            &[],
            &[],
            &[],
            &ReexportMap::new(),
            false,
            false,
            false,
        )?;
        index.modules.push(module_index.module);
    }
    let reexports = reexport_map(&index.modules);
    index.modules.clear();

    let mut all_classes = Vec::new();
    for (file, module) in &project_files {
        let module_index = index_module(
            module,
            file,
            &index.known_modules,
            &config.rules,
            &[],
            &[],
            &[],
            &reexports,
            false,
            false,
            false,
        )?;
        all_classes.extend(module_index.module.classes.clone());
    }

    let mut all_value_bindings = Vec::new();
    let mut all_function_signatures = Vec::new();
    for (file, module) in &project_files {
        let module_index = index_module(
            module,
            file,
            &index.known_modules,
            &config.rules,
            &all_classes,
            &[],
            &[],
            &reexports,
            false,
            false,
            false,
        )?;
        all_value_bindings.extend(module_index.module.value_bindings.clone());
        all_function_signatures.extend(module_index.module.function_signatures.clone());
    }

    all_classes.clear();
    for (file, module) in &project_files {
        let module_index = index_module(
            module,
            file,
            &index.known_modules,
            &config.rules,
            &[],
            &all_value_bindings,
            &[],
            &reexports,
            false,
            false,
            false,
        )?;
        all_classes.extend(module_index.module.classes.clone());
    }

    for (file, module) in &project_files {
        let module_index = index_module(
            module,
            file,
            &index.known_modules,
            &config.rules,
            &all_classes,
            &all_value_bindings,
            &all_function_signatures,
            &reexports,
            is_configured_entrypoint(config, file),
            is_configured_weak_entrypoint(config, file),
            is_test_file(config, file),
        )?;
        index
            .parse_diagnostics
            .extend(module_index.parse_diagnostics);
        index.modules.push(module_index.module);
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

fn reexport_map(modules: &[ModuleIndex]) -> ReexportMap {
    let mut reexports = ReexportMap::new();
    for module in modules {
        for import in &module.imports {
            reexports.insert(
                (module.module.clone(), import.binding.clone()),
                import.target.clone(),
            );
        }
    }
    reexports
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
    available_classes: &[ClassInfo],
    available_values: &[ValueBinding],
    available_fn_sigs: &[FunctionSignature],
    reexports: &ReexportMap,
    is_configured_entrypoint: bool,
    is_configured_weak_entrypoint: bool,
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
    let mut value_bindings = Vec::new();
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
                value_bindings: &mut value_bindings,
                available_classes,
                available_values,
                available_fn_sigs,
                reexports,
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
            value_bindings,
            function_signatures,
            call_argument_types,
            references,
            member_references,
            unresolved_receivers,
            unsupported_expansions,
            is_entrypoint: is_configured_entrypoint
                || (has_main_entrypoint && !is_configured_weak_entrypoint),
            is_weak_entrypoint: is_configured_weak_entrypoint,
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

fn is_configured_weak_entrypoint(config: &LoadedProjectConfig, file: &Path) -> bool {
    config
        .weak_entrypoints
        .iter()
        .any(|entrypoint| configured_path_matches(config, entrypoint, file))
}

fn configured_path_matches(config: &LoadedProjectConfig, pattern: &str, file: &Path) -> bool {
    if !pattern.contains('*') {
        return config.project_dir.join(pattern) == file;
    }
    let relative = file.strip_prefix(&config.project_dir).unwrap_or(file);
    let relative = relative.to_string_lossy().replace('\\', "/");
    glob_pattern_matches(pattern, &relative)
}

fn glob_pattern_matches(pattern: &str, relative: &str) -> bool {
    if pattern == "**/*.py" {
        return relative.ends_with(".py");
    }
    if let Some((prefix, suffix)) = pattern.split_once("**") {
        if !relative.starts_with(prefix) {
            return false;
        }
        let suffix = suffix.trim_start_matches('/');
        if suffix.contains('*') {
            return glob_pattern_matches(suffix, &relative[prefix.len()..]);
        }
        return relative.ends_with(suffix);
    }
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return relative.starts_with(prefix) && relative.ends_with(suffix);
    }
    relative == pattern
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

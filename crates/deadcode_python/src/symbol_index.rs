#[path = "symbol_collector.rs"]
mod symbol_collector;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use deadcode_core::{Diagnostic, Severity, SourceSpan, SymbolKind};
use ruff_text_size::TextRange;

use crate::config::{LoadedProjectConfig, ResolvedRoot, RuleConfig};
use crate::symbol_roots::{is_test_file, root_groups_for_file, root_symbols_for_module};

use self::symbol_collector::SymbolCollector;

pub(crate) type ReexportMap = HashMap<(String, String), ImportTarget>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolIndex {
    pub modules: Vec<ModuleIndex>,
    pub parse_diagnostics: Vec<ParseDiagnostic>,
    pub include_tests: bool,
    pub root_groups: Vec<String>,
    pub counts_as_used_root_groups: Vec<String>,
    pub primary_root_group: String,
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
    pub module_values: Vec<ModuleValue>,
    pub function_signatures: Vec<FunctionSignature>,
    pub pytest_fixtures: Vec<PytestFixture>,
    pub call_argument_types: Vec<CallArgumentType>,
    pub references: Vec<SymbolReference>,
    pub member_references: Vec<MemberReference>,
    pub unresolved_receivers: Vec<UnresolvedReceiver>,
    pub unsupported_expansions: Vec<UnsupportedExpansion>,
    pub is_test: bool,
    pub root_symbols: Vec<RootSymbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootSymbol {
    pub group: String,
    pub symbol: String,
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
pub struct ModuleValue {
    pub qualified_name: String,
    pub name: String,
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
pub struct PytestFixture {
    pub name: String,
    pub function: String,
    pub autouse: bool,
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
    index.primary_root_group = config
        .root_groups
        .first()
        .map(|group| group.name.clone())
        .unwrap_or_else(|| "main".to_string());
    index.root_groups = config
        .root_groups
        .iter()
        .map(|group| group.name.clone())
        .collect();
    index.counts_as_used_root_groups = config
        .root_groups
        .iter()
        .filter(|group| group.counts_as_used)
        .map(|group| group.name.clone())
        .collect();
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
            &index.primary_root_group,
            Vec::new(),
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
            &index.primary_root_group,
            Vec::new(),
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
            &index.primary_root_group,
            Vec::new(),
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
            &index.primary_root_group,
            Vec::new(),
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
            &index.primary_root_group,
            root_groups_for_file(config, file),
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
    index.include_tests = index.include_tests
        || index
            .modules
            .iter()
            .any(|module| module.is_test && !module.root_symbols.is_empty());
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
    primary_root_group: &str,
    configured_root_groups: Vec<String>,
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
    let mut module_values = Vec::new();
    let mut function_signatures = Vec::new();
    let mut pytest_fixtures = Vec::new();
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
                module_values: &mut module_values,
                available_classes,
                available_values,
                available_fn_sigs,
                reexports,
                fn_sigs: &mut function_signatures,
                pytest_fixtures: &mut pytest_fixtures,
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

    let root_symbols = root_symbols_for_module(
        module,
        &symbols,
        &pytest_fixtures,
        primary_root_group,
        configured_root_groups,
        is_test,
        has_main_entrypoint,
    );

    Ok(IndexedModuleResult {
        module: ModuleIndex {
            module: module.to_string(),
            file: file.to_path_buf(),
            symbols,
            imports,
            classes,
            value_bindings,
            module_values,
            function_signatures,
            pytest_fixtures,
            call_argument_types,
            references,
            member_references,
            unresolved_receivers,
            unsupported_expansions,
            is_test,
            root_symbols,
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

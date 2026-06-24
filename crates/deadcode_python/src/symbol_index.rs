use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use deadcode_core::{Diagnostic, Severity, SourceSpan, SymbolKind};
use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use crate::config::{LoadedProjectConfig, ResolvedRoot};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SymbolIndex {
    pub modules: Vec<ModuleIndex>,
    pub parse_diagnostics: Vec<ParseDiagnostic>,
    known_modules: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleIndex {
    pub module: String,
    pub file: PathBuf,
    pub symbols: Vec<IndexedSymbol>,
    pub imports: Vec<ResolvedImport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedSymbol {
    pub qualified_name: String,
    pub name: String,
    pub kind: SymbolKind,
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
            let module_index = index_module(&module, &file, &index.known_modules)?;
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

    index.known_modules.clear();

    Ok(index)
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
                known_modules,
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

    Ok(IndexedModuleResult {
        module: ModuleIndex {
            module: module.to_string(),
            file: file.to_path_buf(),
            symbols,
            imports,
        },
        parse_diagnostics,
    })
}

struct SymbolCollector<'a> {
    module: &'a str,
    file: &'a str,
    locator: &'a SourceLocator,
    symbols: &'a mut Vec<IndexedSymbol>,
    imports: &'a mut Vec<ResolvedImport>,
    known_modules: &'a HashSet<String>,
}

impl SymbolCollector<'_> {
    fn collect_suite(&mut self, suite: &[ast::Stmt]) {
        for statement in suite {
            self.collect_module_statement(statement);
        }
    }

    fn collect_module_statement(&mut self, statement: &ast::Stmt) {
        match statement {
            ast::Stmt::FunctionDef(function) => {
                self.push_symbol(
                    format!("{}.{}", self.module, function.name.as_str()),
                    function.name.as_str(),
                    SymbolKind::Function,
                    function.range,
                );
            }
            ast::Stmt::ClassDef(class_def) => {
                let class_name = class_def.name.as_str();
                self.push_symbol(
                    format!("{}.{}", self.module, class_name),
                    class_name,
                    SymbolKind::Class,
                    class_def.range,
                );
                self.collect_class_body(class_name, &class_def.body);
            }
            ast::Stmt::Import(import) => {
                for alias in &import.names {
                    let target_module = alias.name.as_str().to_string();
                    let binding = alias
                        .asname
                        .as_ref()
                        .map_or_else(|| first_module_segment(&target_module), ToString::to_string);
                    self.push_import(
                        binding,
                        ImportTarget::Module {
                            external: !self.known_modules.contains(&target_module),
                            module: target_module,
                        },
                        import.range,
                    );
                }
            }
            ast::Stmt::ImportFrom(import_from) => {
                let Some(base_module) = self.resolve_import_from_base(import_from) else {
                    return;
                };
                let base_is_external = !self.known_modules.contains(&base_module);
                for alias in &import_from.names {
                    let imported_name = alias.name.as_str();
                    let binding = alias
                        .asname
                        .as_ref()
                        .map_or_else(|| imported_name.to_string(), ToString::to_string);
                    let target = if imported_name == "*" {
                        ImportTarget::Star {
                            external: base_is_external,
                            module: base_module.clone(),
                        }
                    } else {
                        let candidate_module = format!("{base_module}.{imported_name}");
                        if self.known_modules.contains(&candidate_module) {
                            ImportTarget::Module {
                                external: false,
                                module: candidate_module,
                            }
                        } else {
                            ImportTarget::Symbol {
                                external: base_is_external,
                                module: base_module.clone(),
                                name: imported_name.to_string(),
                            }
                        }
                    };
                    self.push_import(binding, target, import_from.range);
                }
            }
            _ => {}
        }
    }

    fn collect_class_body(&mut self, class_name: &str, body: &[ast::Stmt]) {
        for statement in body {
            match statement {
                ast::Stmt::FunctionDef(function) => {
                    let method_name = function.name.as_str();
                    self.push_symbol(
                        format!("{}.{}.{}", self.module, class_name, method_name),
                        method_name,
                        SymbolKind::Method,
                        function.range,
                    );
                    self.collect_self_assignments(class_name, &function.body);
                }
                ast::Stmt::AnnAssign(assign) => {
                    if let Some(name) = target_name(&assign.target) {
                        self.push_symbol(
                            format!("{}.{}.{}", self.module, class_name, name),
                            name,
                            SymbolKind::Field,
                            assign.range,
                        );
                    }
                }
                ast::Stmt::Assign(assign) => {
                    for target in &assign.targets {
                        if let Some(name) = target_name(target) {
                            self.push_symbol(
                                format!("{}.{}.{}", self.module, class_name, name),
                                name,
                                SymbolKind::Attribute,
                                assign.range,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn collect_self_assignments(&mut self, class_name: &str, body: &[ast::Stmt]) {
        for statement in body {
            self.collect_self_assignments_in_statement(class_name, statement);
        }
    }

    fn collect_self_assignments_in_statement(&mut self, class_name: &str, statement: &ast::Stmt) {
        match statement {
            ast::Stmt::Assign(assign) => {
                for target in &assign.targets {
                    if let Some(name) = self_attribute_name(target) {
                        self.push_symbol(
                            format!("{}.{}.{}", self.module, class_name, name),
                            name,
                            SymbolKind::Attribute,
                            assign.range,
                        );
                    }
                }
            }
            ast::Stmt::AnnAssign(assign) => {
                if let Some(name) = self_attribute_name(&assign.target) {
                    self.push_symbol(
                        format!("{}.{}.{}", self.module, class_name, name),
                        name,
                        SymbolKind::Field,
                        assign.range,
                    );
                }
            }
            ast::Stmt::If(if_stmt) => {
                for nested in &if_stmt.body {
                    self.collect_self_assignments_in_statement(class_name, nested);
                }
                for clause in &if_stmt.elif_else_clauses {
                    for nested in &clause.body {
                        self.collect_self_assignments_in_statement(class_name, nested);
                    }
                }
            }
            _ => {}
        }
    }

    fn push_symbol(
        &mut self,
        qualified_name: String,
        name: &str,
        kind: SymbolKind,
        range: TextRange,
    ) {
        self.symbols.push(IndexedSymbol {
            qualified_name,
            name: name.to_string(),
            kind,
            span: self.locator.span_from_range_string(self.file, range),
        });
    }

    fn push_import(&mut self, binding: String, target: ImportTarget, range: TextRange) {
        self.imports.push(ResolvedImport {
            binding,
            target,
            span: self.locator.span_from_range_string(self.file, range),
        });
    }

    fn resolve_import_from_base(&self, import_from: &ast::StmtImportFrom) -> Option<String> {
        let imported_module = import_from.module.as_ref().map(ast::Identifier::as_str);
        if import_from.level == 0 {
            return imported_module.map(ToString::to_string);
        }

        let mut parts = self.module.split('.').collect::<Vec<_>>();
        parts.pop();
        let ancestor_count = import_from.level.saturating_sub(1) as usize;
        if ancestor_count > parts.len() {
            return None;
        }
        parts.truncate(parts.len() - ancestor_count);
        if let Some(imported_module) = imported_module {
            parts.extend(imported_module.split('.'));
        }
        Some(parts.join("."))
    }
}

fn first_module_segment(module: &str) -> String {
    module.split('.').next().unwrap_or(module).to_string()
}

fn target_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        _ => None,
    }
}

fn self_attribute_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Attribute(attribute) => match attribute.value.as_ref() {
            ast::Expr::Name(name) if name.id.as_str() == "self" => Some(attribute.attr.as_str()),
            _ => None,
        },
        _ => None,
    }
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
mod tests {
    use std::fs;

    use crate::config::{LoadedProjectConfig, ResolvedRoot};

    use super::*;

    #[test]
    fn indexes_module_functions_classes_methods_and_fields() {
        let workspace = test_workspace("indexes_module_functions_classes_methods_and_fields");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("service.py"),
            r#"
def module_function():
    pass

class ExampleEntity:
    class_attr = 1
    name: str

    def save(self):
        self.saved = True

    def configure(self):
        if True:
            self.flag: bool = False
"#,
        )
        .unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")]);

        let index = index_project(&config).unwrap();
        let symbols = index
            .modules
            .iter()
            .flat_map(|module| module.symbols.iter())
            .map(|symbol| (symbol.qualified_name.as_str(), symbol.kind.clone()))
            .collect::<Vec<_>>();

        assert!(symbols.contains(&("pkg.service.module_function", SymbolKind::Function)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity", SymbolKind::Class)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity.save", SymbolKind::Method)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity.configure", SymbolKind::Method)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity.class_attr", SymbolKind::Attribute)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity.name", SymbolKind::Field)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity.saved", SymbolKind::Attribute)));
        assert!(symbols.contains(&("pkg.service.ExampleEntity.flag", SymbolKind::Field)));
    }

    #[test]
    fn indexes_package_init_as_package_module() {
        let workspace = test_workspace("indexes_package_init_as_package_module");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("__init__.py"), "def exported():\n    pass\n").unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")]);

        let index = index_project(&config).unwrap();

        assert_eq!(index.modules[0].module, "pkg");
        assert_eq!(index.modules[0].symbols[0].qualified_name, "pkg");
        assert!(index.modules[0]
            .symbols
            .iter()
            .any(|symbol| symbol.qualified_name == "pkg.exported"));
    }

    #[test]
    fn parse_errors_become_diagnostics() {
        let workspace = test_workspace("parse_errors_become_diagnostics");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("broken.py"), "def broken(:\n").unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")]);

        let index = index_project(&config).unwrap();

        assert_eq!(index.parse_diagnostics.len(), 1);
        assert!(index.parse_diagnostics[0]
            .message
            .contains("could not parse"));
    }

    #[test]
    fn resolves_absolute_and_relative_local_imports() {
        let workspace = test_workspace("resolves_absolute_and_relative_local_imports");
        let package = workspace.join("pkg");
        fs::create_dir_all(package.join("sub")).unwrap();
        fs::write(package.join("__init__.py"), "").unwrap();
        fs::write(package.join("helpers.py"), "def help_me():\n    pass\n").unwrap();
        fs::write(package.join("sub/__init__.py"), "").unwrap();
        fs::write(
            package.join("sub/feature.py"),
            "import pkg.helpers as helpers\nfrom ..helpers import help_me\n",
        )
        .unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")]);

        let index = index_project(&config).unwrap();
        let feature = module(&index, "pkg.sub.feature");

        assert_eq!(feature.imports.len(), 2);
        assert_eq!(feature.imports[0].binding, "helpers");
        assert_eq!(
            feature.imports[0].target,
            ImportTarget::Module {
                module: "pkg.helpers".to_string(),
                external: false
            }
        );
        assert_eq!(feature.imports[1].binding, "help_me");
        assert_eq!(
            feature.imports[1].target,
            ImportTarget::Symbol {
                module: "pkg.helpers".to_string(),
                name: "help_me".to_string(),
                external: false
            }
        );
    }

    #[test]
    fn preserves_external_import_identities() {
        let workspace = test_workspace("preserves_external_import_identities");
        let package = workspace.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("app.py"),
            "from fastapi import APIRouter\nimport pydantic.fields\n",
        )
        .unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")]);

        let index = index_project(&config).unwrap();
        let app = module(&index, "pkg.app");

        assert_eq!(app.imports.len(), 2);
        assert_eq!(app.imports[0].binding, "APIRouter");
        assert_eq!(
            app.imports[0].target,
            ImportTarget::Symbol {
                module: "fastapi".to_string(),
                name: "APIRouter".to_string(),
                external: true
            }
        );
        assert_eq!(app.imports[1].binding, "pydantic");
        assert_eq!(
            app.imports[1].target,
            ImportTarget::Module {
                module: "pydantic.fields".to_string(),
                external: true
            }
        );
    }

    #[test]
    fn resolves_imported_submodule_before_symbol() {
        let workspace = test_workspace("resolves_imported_submodule_before_symbol");
        let package = workspace.join("pkg");
        fs::create_dir_all(package.join("models")).unwrap();
        fs::write(package.join("__init__.py"), "").unwrap();
        fs::write(package.join("models/__init__.py"), "").unwrap();
        fs::write(package.join("models/entity.py"), "class ExampleEntity:\n    pass\n").unwrap();
        fs::write(package.join("consumer.py"), "from pkg.models import entity\n").unwrap();
        let config = loaded_config(&workspace, vec![root(&package, "pkg")]);

        let index = index_project(&config).unwrap();
        let consumer = module(&index, "pkg.consumer");

        assert_eq!(
            consumer.imports[0].target,
            ImportTarget::Module {
                module: "pkg.models.entity".to_string(),
                external: false
            }
        );
    }

    fn loaded_config(workspace: &Path, roots: Vec<ResolvedRoot>) -> LoadedProjectConfig {
        LoadedProjectConfig {
            config_path: workspace.join("dead-code-finder.json"),
            project_dir: workspace.to_path_buf(),
            roots,
            entrypoints: Vec::new(),
            include_tests: false,
            test_patterns: Vec::new(),
        }
    }

    fn root(path: &Path, module: &str) -> ResolvedRoot {
        ResolvedRoot {
            path: path.to_path_buf(),
            module: module.to_string(),
        }
    }

    fn module<'a>(index: &'a SymbolIndex, name: &str) -> &'a ModuleIndex {
        index
            .modules
            .iter()
            .find(|module| module.module == name)
            .unwrap()
    }

    fn test_workspace(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("deadcode_symbol_index_{name}_{unique}"));
        fs::create_dir_all(&path).unwrap();
        path
    }
}

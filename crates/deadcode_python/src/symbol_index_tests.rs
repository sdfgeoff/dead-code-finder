use super::*;

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
            weak_entrypoints: Vec::new(),
            include_tests: false,
            test_patterns: Vec::new(),
            rules: crate::config::RuleConfig::default(),
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

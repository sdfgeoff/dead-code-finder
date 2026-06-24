#[path = "symbol_aliases.rs"]
mod symbol_aliases;
#[path = "symbol_append.rs"]
mod symbol_append;
#[path = "symbol_branch_narrowing.rs"]
mod symbol_branch_narrowing;
#[path = "symbol_branch_types.rs"]
mod symbol_branch_types;
#[path = "symbol_comprehension_narrowing.rs"]
mod symbol_comprehension_narrowing;
#[path = "symbol_comprehension_typeflow.rs"]
mod symbol_comprehension_typeflow;
#[path = "symbol_construction.rs"]
mod symbol_construction;
#[path = "symbol_context.rs"]
mod symbol_context;
#[path = "symbol_expansion.rs"]
mod symbol_expansion;
#[path = "symbol_expr.rs"]
mod symbol_expr;
#[path = "symbol_external.rs"]
mod symbol_external;
#[path = "symbol_external_flow.rs"]
mod symbol_external_flow;
#[path = "symbol_fields.rs"]
mod symbol_fields;
#[path = "symbol_generics.rs"]
mod symbol_generics;
#[path = "symbol_imports.rs"]
mod symbol_imports;
#[path = "symbol_interpolation.rs"]
mod symbol_interpolation;
#[path = "symbol_iteration.rs"]
mod symbol_iteration;
#[path = "symbol_lambda.rs"]
mod symbol_lambda;
#[path = "symbol_member_refs.rs"]
mod symbol_member_refs;
#[path = "symbol_members.rs"]
mod symbol_members;
#[path = "symbol_metadata.rs"]
mod symbol_metadata;
#[path = "symbol_references.rs"]
mod symbol_references;
#[path = "symbol_rules.rs"]
mod symbol_rules;
#[path = "symbol_typeflow.rs"]
mod symbol_typeflow;
#[path = "symbol_types.rs"]
mod symbol_types;
#[path = "symbol_typevars.rs"]
mod symbol_typevars;

use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;
use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use self::symbol_expr::{is_main_guard, target_name};
use self::symbol_fields::collect_self_assignments;
use self::symbol_imports::{collect_import, collect_import_from};
use self::symbol_metadata::{class_info, function_signature};
use self::symbol_rules::decorator_registers_function;
use self::symbol_types::type_binding_from_annotation_expr;
use super::{
    CallArgumentType, ClassInfo, FunctionSignature, IndexedSymbol, MemberReference, ResolvedImport,
    SourceLocator, SymbolReference, TypeBinding, UnresolvedReceiver, UnsupportedExpansion,
    ValueBinding,
};
use crate::config::RuleConfig;
use crate::symbol_index::ReexportMap;

pub(super) struct SymbolCollector<'a> {
    pub(super) module: &'a str,
    pub(super) file: &'a str,
    pub(super) locator: &'a SourceLocator,
    pub(super) symbols: &'a mut Vec<IndexedSymbol>,
    pub(super) imports: &'a mut Vec<ResolvedImport>,
    pub(super) classes: &'a mut Vec<ClassInfo>,
    pub(super) value_bindings: &'a mut Vec<ValueBinding>,
    pub(super) available_classes: &'a [ClassInfo],
    pub(super) available_values: &'a [ValueBinding],
    pub(super) available_fn_sigs: &'a [FunctionSignature],
    pub(super) fn_sigs: &'a mut Vec<FunctionSignature>,
    pub(super) call_args: &'a mut Vec<CallArgumentType>,
    pub(super) references: &'a mut Vec<SymbolReference>,
    pub(super) member_refs: &'a mut Vec<MemberReference>,
    pub(super) unresolved_receivers: &'a mut Vec<UnresolvedReceiver>,
    pub(super) unsupported: &'a mut Vec<UnsupportedExpansion>,
    pub(super) main_entry: &'a mut bool,
    pub(super) known_modules: &'a HashSet<String>,
    pub(super) reexports: &'a ReexportMap,
    pub(super) rules: &'a RuleConfig,
}

impl SymbolCollector<'_> {
    pub(super) fn collect_suite(&mut self, suite: &[ast::Stmt]) {
        let mut module_types = HashMap::new();
        for statement in suite {
            self.collect_module_statement(statement, &mut module_types);
        }
    }

    fn collect_module_statement(
        &mut self,
        statement: &ast::Stmt,
        module_types: &mut HashMap<String, TypeBinding>,
    ) {
        match statement {
            ast::Stmt::FunctionDef(function) => {
                let function_owner = format!("{}.{}", self.module, function.name.as_str());
                self.push_symbol(
                    function_owner.clone(),
                    function.name.as_str(),
                    SymbolKind::Function,
                    function.range,
                );
                let mut types = module_types.clone();
                types.extend(self.function_type_bindings(function, None, module_types));
                self.push_function_signature(&function_owner, function, &types);
                self.collect_function_annotation_references(&function_owner, function);
                self.collect_decorator_rules(function, module_types);
                self.collect_function_references(&function_owner, function, types);
            }
            ast::Stmt::ClassDef(class_def) => {
                let class_name = class_def.name.as_str();
                self.push_symbol(
                    format!("{}.{}", self.module, class_name),
                    class_name,
                    SymbolKind::Class,
                    class_def.range,
                );
                self.push_class_info(format!("{}.{}", self.module, class_name), class_def);
                self.collect_class_body(class_name, &class_def.body, module_types);
            }
            ast::Stmt::Import(import) => {
                collect_import(
                    self.file,
                    self.locator,
                    self.imports,
                    self.known_modules,
                    import,
                );
            }
            ast::Stmt::ImportFrom(import_from) => {
                let import_start = self.imports.len();
                collect_import_from(
                    self.module,
                    self.file,
                    self.locator,
                    self.imports,
                    self.known_modules,
                    self.reexports,
                    import_from,
                );
                self.push_imported_value_bindings(module_types, import_start);
            }
            ast::Stmt::If(if_stmt) if is_main_guard(if_stmt) => {
                *self.main_entry = true;
                self.collect_statement_references(self.module, statement, module_types);
            }
            statement => {
                self.collect_statement_references(self.module, statement, module_types);
            }
        }
    }

    fn collect_class_body(
        &mut self,
        class_name: &str,
        body: &[ast::Stmt],
        module_types: &HashMap<String, TypeBinding>,
    ) {
        for statement in body {
            match statement {
                ast::Stmt::FunctionDef(function) => {
                    let method_name = function.name.as_str();
                    let method_owner = format!("{}.{}.{}", self.module, class_name, method_name);
                    self.push_symbol(
                        method_owner.clone(),
                        method_name,
                        SymbolKind::Method,
                        function.range,
                    );
                    collect_self_assignments(
                        self.module,
                        self.file,
                        self.locator,
                        self.symbols,
                        class_name,
                        &function.body,
                    );
                    let mut types = module_types.clone();
                    types.extend(self.function_type_bindings(
                        function,
                        Some(class_name),
                        module_types,
                    ));
                    self.push_function_signature(&method_owner, function, &types);
                    self.collect_function_annotation_references(&method_owner, function);
                    self.collect_function_references(&method_owner, function, types);
                }
                ast::Stmt::AnnAssign(assign) => {
                    if let Some(name) = target_name(&assign.target) {
                        self.push_symbol(
                            format!("{}.{}.{}", self.module, class_name, name),
                            name,
                            SymbolKind::Field,
                            assign.range,
                        );
                        let class_owner = format!("{}.{}", self.module, class_name);
                        self.collect_expr_references(
                            &class_owner,
                            &assign.annotation,
                            module_types,
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

    fn push_class_info(&mut self, class: String, class_def: &ast::StmtClassDef) {
        self.classes.push(class_info(
            self.module,
            self.imports,
            class,
            class_def,
            self.available_values,
        ));
    }

    fn push_function_signature(
        &mut self,
        function: &str,
        function_def: &ast::StmtFunctionDef,
        types: &HashMap<String, TypeBinding>,
    ) {
        let mut signature = function_signature(self.module, self.imports, function, function_def);
        if signature.return_type.is_none() {
            signature.return_type = self.inferred_function_return(function_def, types);
        }
        self.fn_sigs.push(signature);
    }

    fn inferred_function_return(
        &self,
        function_def: &ast::StmtFunctionDef,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let mut inferred = None;
        for statement in &function_def.body {
            let ast::Stmt::Return(return_stmt) = statement else {
                continue;
            };
            let Some(value) = &return_stmt.value else {
                return None;
            };
            let binding = self.assignment_value_binding(value, types)?;
            if inferred
                .as_ref()
                .is_some_and(|existing: &TypeBinding| existing != &binding)
            {
                return None;
            }
            inferred = Some(binding);
        }
        inferred
    }

    fn push_reference(&mut self, from: &str, name: &str, range: TextRange) {
        self.references.push(SymbolReference {
            from: from.to_string(),
            name: name.to_string(),
            span: self.locator.span_from_range_string(self.file, range),
        });
    }

    fn collect_decorator_rules(
        &mut self,
        function: &ast::StmtFunctionDef,
        types: &HashMap<String, TypeBinding>,
    ) {
        for decorator in &function.decorator_list {
            if decorator_registers_function(self.rules, &decorator.expression, types) {
                self.push_reference(self.module, function.name.as_str(), decorator.range);
            }
            self.collect_expr_references(self.module, &decorator.expression, types);
        }
    }

    fn collect_function_references(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
        mut types: HashMap<String, TypeBinding>,
    ) {
        for parameter in function.parameters.iter_non_variadic_params() {
            if let Some(default) = parameter.default() {
                self.collect_expr_references(owner, default, &types);
            }
        }
        for statement in &function.body {
            self.collect_statement_references(owner, statement, &mut types);
        }
    }

    fn collect_function_annotation_references(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
    ) {
        let types = HashMap::new();
        for parameter in function.parameters.iter() {
            if let Some(annotation) = parameter.as_parameter().annotation() {
                self.collect_expr_references(owner, annotation, &types);
            }
        }
        if let Some(returns) = &function.returns {
            self.collect_expr_references(owner, returns, &types);
        }
    }

    fn function_type_bindings(
        &self,
        function: &ast::StmtFunctionDef,
        class_name: Option<&str>,
        module_types: &HashMap<String, TypeBinding>,
    ) -> HashMap<String, TypeBinding> {
        let mut types = HashMap::new();
        if let Some(class_name) = class_name {
            types.insert(
                "self".to_string(),
                TypeBinding::erased(format!("{}.{}", self.module, class_name)),
            );
            types.insert(
                "cls".to_string(),
                TypeBinding::erased(format!("{}.{}", self.module, class_name)),
            );
        }
        for parameter in function.parameters.iter() {
            let parameter = parameter.as_parameter();
            if let Some(annotation) = parameter.annotation() {
                if let Some(type_name) = type_binding_from_annotation(
                    self.module,
                    self.imports,
                    annotation,
                    module_types,
                ) {
                    types.insert(parameter.name.as_str().to_string(), type_name);
                }
            }
        }
        types
    }
}

fn type_binding_from_annotation(
    module: &str,
    imports: &[ResolvedImport],
    annotation: &ast::Expr,
    module_types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    if let ast::Expr::Name(name) = annotation {
        if let Some(binding) = module_types.get(name.id.as_str()) {
            return Some(binding.clone());
        }
    }
    type_binding_from_annotation_expr(module, imports, annotation)
}

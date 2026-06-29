#[path = "symbol_aliases.rs"]
mod symbol_aliases;
#[path = "symbol_append.rs"]
mod symbol_append;
#[path = "symbol_assignment_target.rs"]
mod symbol_assignment_target;
#[path = "symbol_binary_ops.rs"]
mod symbol_binary_ops;
#[path = "symbol_branch_narrowing.rs"]
mod symbol_branch_narrowing;
#[path = "symbol_branch_types.rs"]
mod symbol_branch_types;
#[path = "symbol_call_args.rs"]
mod symbol_call_args;
#[path = "symbol_call_references.rs"]
mod symbol_call_references;
#[path = "symbol_callable_alias.rs"]
mod symbol_callable_alias;
#[path = "symbol_class_definition.rs"]
mod symbol_class_definition;
#[path = "symbol_comprehension_narrowing.rs"]
mod symbol_comprehension_narrowing;
#[path = "symbol_comprehension_typeflow.rs"]
mod symbol_comprehension_typeflow;
#[path = "symbol_construction.rs"]
mod symbol_construction;
#[path = "symbol_context.rs"]
mod symbol_context;
#[path = "symbol_datetime.rs"]
mod symbol_datetime;
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
#[path = "symbol_function_signature.rs"]
mod symbol_function_signature;
#[path = "symbol_generics.rs"]
mod symbol_generics;
#[path = "symbol_imports.rs"]
mod symbol_imports;
#[path = "symbol_interpolation.rs"]
mod symbol_interpolation;
#[path = "symbol_iteration.rs"]
mod symbol_iteration;
#[path = "symbol_json_types.rs"]
mod symbol_json_types;
#[path = "symbol_lambda.rs"]
mod symbol_lambda;
#[path = "symbol_local_functions.rs"]
mod symbol_local_functions;
#[path = "symbol_mapping_types.rs"]
mod symbol_mapping_types;
#[path = "symbol_member_refs.rs"]
mod symbol_member_refs;
#[path = "symbol_members.rs"]
mod symbol_members;
#[path = "symbol_metadata.rs"]
mod symbol_metadata;
#[path = "symbol_model_surface.rs"]
mod symbol_model_surface;
#[path = "symbol_pydantic.rs"]
mod symbol_pydantic;
#[path = "symbol_references.rs"]
mod symbol_references;
#[path = "symbol_rules.rs"]
mod symbol_rules;
#[path = "symbol_statement_flow.rs"]
mod symbol_statement_flow;
#[path = "symbol_typed_dict.rs"]
mod symbol_typed_dict;
#[path = "symbol_typeflow.rs"]
mod symbol_typeflow;
#[path = "symbol_types.rs"]
mod symbol_types;
#[path = "symbol_typevars.rs"]
mod symbol_typevars;
#[path = "symbol_unions.rs"]
mod symbol_unions;
#[path = "symbol_validated_returns.rs"]
mod symbol_validated_returns;
#[path = "symbol_value_bindings.rs"]
mod symbol_value_bindings;

use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;
use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use self::symbol_expr::{is_main_guard, target_name};
use self::symbol_fields::collect_self_assignments;
use self::symbol_imports::{collect_import, collect_import_from};
use self::symbol_members::push_member_reference;
use self::symbol_metadata::class_info;
use self::symbol_rules::{
    decorator_callable_wrapper_type, decorator_marks_boundary_function,
    decorator_registers_function,
};
use super::{
    AccessKind, CallArgumentType, ClassInfo, FunctionSignature, IndexedSymbol, MemberReference,
    PytestFixture, ResolvedImport, SourceLocator, SymbolReference, TypeBinding, UnresolvedReceiver,
    UnsupportedExpansion, ValueBinding,
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
    pub(super) pytest_fixtures: &'a mut Vec<PytestFixture>,
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
                self.collect_decorator_rules(self.module, function, module_types);
                self.collect_function_references(&function_owner, function, types);
            }
            ast::Stmt::ClassDef(class_def) => {
                let class_name = class_def.name.as_str();
                let class_owner = format!("{}.{}", self.module, class_name);
                self.push_symbol(
                    class_owner.clone(),
                    class_name,
                    SymbolKind::Class,
                    class_def.range,
                );
                self.push_class_info(class_owner.clone(), class_def);
                self.collect_class_definition_references(&class_owner, class_def, module_types);
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
                    let class_owner = format!("{}.{}", self.module, class_name);
                    self.collect_decorator_rules(&class_owner, function, &types);
                    self.collect_function_references(&method_owner, function, types);
                }
                ast::Stmt::AnnAssign(assign) => {
                    let class_owner = format!("{}.{}", self.module, class_name);
                    if let Some(name) = target_name(&assign.target) {
                        self.push_symbol(
                            format!("{}.{}.{}", self.module, class_name, name),
                            name,
                            SymbolKind::Field,
                            assign.range,
                        );
                        self.collect_expr_references(
                            &class_owner,
                            &assign.annotation,
                            module_types,
                        );
                    }
                    if let Some(value) = &assign.value {
                        self.collect_expr_references(&class_owner, value, module_types);
                    }
                }
                ast::Stmt::Assign(assign) => {
                    let class_owner = format!("{}.{}", self.module, class_name);
                    self.collect_expr_references(&class_owner, &assign.value, module_types);
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

    fn push_reference(&mut self, from: &str, name: &str, range: TextRange) {
        self.references.push(SymbolReference {
            from: from.to_string(),
            name: name.to_string(),
            span: self.locator.span_from_range_string(self.file, range),
        });
    }

    fn collect_decorator_rules(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
        types: &HashMap<String, TypeBinding>,
    ) {
        let function_owner = owner
            .strip_prefix(self.module)
            .and_then(|suffix| {
                suffix
                    .strip_prefix('.')
                    .map(|_| format!("{owner}.{}", function.name.as_str()))
            })
            .unwrap_or_else(|| format!("{}.{}", self.module, function.name.as_str()));
        for decorator in &function.decorator_list {
            if let Some(fixture) =
                pytest_fixture_from_decorator(&function_owner, function, &decorator.expression)
            {
                self.pytest_fixtures.push(fixture);
            }
            if decorator_registers_function(
                self.module,
                self.imports,
                self.rules,
                &decorator.expression,
                types,
            ) {
                let name = owner.strip_prefix(self.module).and_then(|suffix| {
                    suffix
                        .strip_prefix('.')
                        .map(|class_name| format!("{class_name}.{}", function.name.as_str()))
                });
                self.push_reference(
                    owner,
                    name.as_deref().unwrap_or(function.name.as_str()),
                    decorator.range,
                );
            }
            if decorator_marks_boundary_function(
                self.module,
                self.imports,
                self.rules,
                &decorator.expression,
                types,
            ) {
                self.collect_boundary_function_model_references(
                    &function_owner,
                    function,
                    decorator.range,
                );
            }
            if let Some(callable_type) = decorator_callable_wrapper_type(
                self.module,
                self.imports,
                self.rules,
                &decorator.expression,
                types,
            ) {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    &function_owner,
                    format!("{callable_type}.__call__"),
                    AccessKind::Call,
                    decorator.range,
                );
            }
            self.collect_expr_references(owner, &decorator.expression, types);
            self.collect_expr_references(&function_owner, &decorator.expression, types);
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
}

fn pytest_fixture_from_decorator(
    function_owner: &str,
    function: &ast::StmtFunctionDef,
    decorator: &ast::Expr,
) -> Option<PytestFixture> {
    let (callee, call) = match decorator {
        ast::Expr::Call(call) => (call.func.as_ref(), Some(call)),
        expr => (expr, None),
    };
    if !is_pytest_fixture_callee(callee) {
        return None;
    }
    Some(PytestFixture {
        name: call
            .and_then(pytest_fixture_name)
            .unwrap_or_else(|| function.name.as_str().to_string()),
        function: function_owner.to_string(),
        autouse: call.is_some_and(pytest_fixture_is_autouse),
    })
}

fn is_pytest_fixture_callee(callee: &ast::Expr) -> bool {
    match callee {
        ast::Expr::Name(name) => name.id.as_str() == "fixture",
        ast::Expr::Attribute(attribute) => {
            attribute.attr.as_str() == "fixture"
                && matches!(
                    attribute.value.as_ref(),
                    ast::Expr::Name(receiver) if receiver.id.as_str() == "pytest"
                )
        }
        _ => false,
    }
}

fn pytest_fixture_name(call: &ast::ExprCall) -> Option<String> {
    call.arguments
        .keywords
        .iter()
        .find(|keyword| {
            keyword
                .arg
                .as_ref()
                .is_some_and(|arg| arg.as_str() == "name")
        })
        .and_then(|keyword| string_literal(&keyword.value))
}

fn pytest_fixture_is_autouse(call: &ast::ExprCall) -> bool {
    call.arguments
        .keywords
        .iter()
        .find(|keyword| {
            keyword
                .arg
                .as_ref()
                .is_some_and(|arg| arg.as_str() == "autouse")
        })
        .is_some_and(|keyword| {
            matches!(
                &keyword.value,
                ast::Expr::BooleanLiteral(ast::ExprBooleanLiteral { value: true, .. })
            )
        })
}

fn string_literal(expr: &ast::Expr) -> Option<String> {
    let ast::Expr::StringLiteral(string) = expr else {
        return None;
    };
    Some(string.value.to_str().to_string())
}

#[path = "symbol_expr.rs"]
mod symbol_expr;
#[path = "symbol_fields.rs"]
mod symbol_fields;
#[path = "symbol_generics.rs"]
mod symbol_generics;
#[path = "symbol_imports.rs"]
mod symbol_imports;
#[path = "symbol_members.rs"]
mod symbol_members;
#[path = "symbol_metadata.rs"]
mod symbol_metadata;
#[path = "symbol_rules.rs"]
mod symbol_rules;
#[path = "symbol_types.rs"]
mod symbol_types;

use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;
use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange};

use self::symbol_expr::{is_main_guard, target_name};
use self::symbol_fields::collect_self_assignments;
use self::symbol_generics::field_read_type;
use self::symbol_imports::{collect_import, collect_import_from};
use self::symbol_members::push_member_reference;
use self::symbol_metadata::{class_info, function_signature};
use self::symbol_rules::{
    callable_argument_references, callable_identity, constructed_type_from_callee,
    constructor_binding, decorator_registers_function,
};
use self::symbol_types::{type_binding_from_expr, TypeBinding};
use super::{
    AccessKind, CallArgumentType, ClassInfo, FunctionSignature, ImportTarget, IndexedSymbol,
    MemberReference, ResolvedImport, SourceLocator, SymbolReference, UnresolvedReceiver,
    UnsupportedExpansion,
};
use crate::config::RuleConfig;

pub(super) struct SymbolCollector<'a> {
    pub(super) module: &'a str,
    pub(super) file: &'a str,
    pub(super) locator: &'a SourceLocator,
    pub(super) symbols: &'a mut Vec<IndexedSymbol>,
    pub(super) imports: &'a mut Vec<ResolvedImport>,
    pub(super) classes: &'a mut Vec<ClassInfo>,
    pub(super) fn_sigs: &'a mut Vec<FunctionSignature>,
    pub(super) call_args: &'a mut Vec<CallArgumentType>,
    pub(super) references: &'a mut Vec<SymbolReference>,
    pub(super) member_refs: &'a mut Vec<MemberReference>,
    pub(super) unresolved_receivers: &'a mut Vec<UnresolvedReceiver>,
    pub(super) unsupported: &'a mut Vec<UnsupportedExpansion>,
    pub(super) main_entry: &'a mut bool,
    pub(super) known_modules: &'a HashSet<String>,
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
                self.push_function_signature(&function_owner, function);
                self.collect_decorator_rules(function, module_types);
                let types = self.function_type_bindings(function, None);
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
                self.collect_class_body(class_name, &class_def.body);
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
                collect_import_from(
                    self.module,
                    self.file,
                    self.locator,
                    self.imports,
                    self.known_modules,
                    import_from,
                );
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

    fn collect_class_body(&mut self, class_name: &str, body: &[ast::Stmt]) {
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
                    self.push_function_signature(&method_owner, function);
                    collect_self_assignments(
                        self.module,
                        self.file,
                        self.locator,
                        self.symbols,
                        class_name,
                        &function.body,
                    );
                    let types = self.function_type_bindings(function, Some(class_name));
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
        self.classes
            .push(class_info(self.module, self.imports, class, class_def));
    }

    fn push_function_signature(&mut self, function: &str, function_def: &ast::StmtFunctionDef) {
        self.fn_sigs.push(function_signature(
            self.module,
            self.imports,
            function,
            function_def,
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

    fn collect_statement_references(
        &mut self,
        owner: &str,
        statement: &ast::Stmt,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        match statement {
            ast::Stmt::FunctionDef(function) => {
                let function_owner = format!("{}.{}", self.module, function.name.as_str());
                let types = self.function_type_bindings(function, None);
                self.collect_function_references(&function_owner, function, types);
            }
            ast::Stmt::ClassDef(_) => {}
            ast::Stmt::Expr(expr) => self.collect_expr_references(owner, &expr.value, types),
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.collect_expr_references(owner, value, types);
                }
            }
            ast::Stmt::Assign(assign) => {
                self.collect_expr_references(owner, &assign.value, types);
                for target in &assign.targets {
                    self.collect_assignment_target(owner, target, types);
                }
                if let Some(type_name) =
                    constructor_binding(self.module, self.imports, self.rules, &assign.value)
                {
                    for target in &assign.targets {
                        if let Some(name) = target_name(target) {
                            types.insert(name.to_string(), type_name.clone());
                        }
                    }
                }
                if let Some(field_type) = field_read_type(self.classes, &assign.value, types) {
                    for target in &assign.targets {
                        if let Some(name) = target_name(target) {
                            types.insert(name.to_string(), field_type.clone());
                        }
                    }
                }
            }
            ast::Stmt::AnnAssign(assign) => {
                if let Some(name) = target_name(&assign.target) {
                    if let Some(type_name) =
                        type_binding_from_expr(self.module, self.imports, &assign.annotation)
                    {
                        types.insert(name.to_string(), type_name);
                    }
                } else {
                    self.collect_assignment_target(owner, &assign.target, types);
                }
                if let Some(value) = &assign.value {
                    self.collect_expr_references(owner, value, types);
                }
            }
            ast::Stmt::If(if_stmt) => {
                self.collect_expr_references(owner, &if_stmt.test, types);
                for nested in &if_stmt.body {
                    self.collect_statement_references(owner, nested, types);
                }
                for clause in &if_stmt.elif_else_clauses {
                    if let Some(test) = &clause.test {
                        self.collect_expr_references(owner, test, types);
                    }
                    for nested in &clause.body {
                        self.collect_statement_references(owner, nested, types);
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_expr_references(
        &mut self,
        owner: &str,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        match expr {
            ast::Expr::Name(name) => self.push_reference(owner, name.id.as_str(), name.range),
            ast::Expr::Call(call) => self.collect_call_references(owner, call, types),
            ast::Expr::Attribute(attribute) => {
                self.collect_member_reference(owner, attribute, AccessKind::Read, types);
                self.collect_expr_references(owner, &attribute.value, types);
            }
            ast::Expr::BinOp(bin_op) => {
                self.collect_expr_references(owner, &bin_op.left, types);
                self.collect_expr_references(owner, &bin_op.right, types);
            }
            ast::Expr::Compare(compare) => {
                self.collect_expr_references(owner, &compare.left, types);
                for comparator in compare.comparators.iter() {
                    self.collect_expr_references(owner, comparator, types);
                }
            }
            ast::Expr::BoolOp(bool_op) => {
                for value in &bool_op.values {
                    self.collect_expr_references(owner, value, types);
                }
            }
            _ => {}
        }
    }

    fn collect_call_references(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) {
        if let ast::Expr::Attribute(attribute) = call.func.as_ref() {
            self.collect_member_reference(owner, attribute, AccessKind::Call, types);
            self.collect_expr_references(owner, &attribute.value, types);
        } else {
            self.collect_expr_references(owner, &call.func, types);
        }

        let callee = callable_identity(self.module, self.imports, &call.func);
        for (name, range) in
            callable_argument_references(self.rules, call, callee.as_deref(), types)
        {
            self.push_reference(owner, &name, range);
        }
        for (position, arg) in call.arguments.args.iter().enumerate() {
            if let (Some(callee), Some(concrete_type)) = (
                callee.as_ref(),
                constructor_binding(self.module, self.imports, self.rules, arg),
            ) {
                self.call_args.push(CallArgumentType {
                    from: owner.to_string(),
                    callee: callee.clone(),
                    position,
                    concrete_type: concrete_type.base,
                    span: self.locator.span_from_range_string(self.file, arg.range()),
                });
            }
            self.collect_expr_references(owner, arg, types);
        }
        let constructor_type =
            constructed_type_from_callee(self.module, self.imports, self.rules, &call.func);
        for keyword in &call.arguments.keywords {
            self.collect_expr_references(owner, &keyword.value, types);
            let Some(constructor_type) = constructor_type.as_ref() else {
                continue;
            };
            if let Some(arg) = &keyword.arg {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    owner,
                    format!("{constructor_type}.{}", arg.as_str()),
                    AccessKind::Construct,
                    keyword.range,
                );
            } else {
                self.unsupported.push(UnsupportedExpansion {
                    from: owner.to_string(),
                    target: constructor_type.clone(),
                    span: self
                        .locator
                        .span_from_range_string(self.file, keyword.range),
                });
            }
        }
    }

    fn collect_assignment_target(
        &mut self,
        owner: &str,
        target: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        match target {
            ast::Expr::Attribute(attribute) => {
                self.collect_member_reference(owner, attribute, AccessKind::Write, types);
                self.collect_expr_references(owner, &attribute.value, types);
            }
            ast::Expr::Tuple(tuple) => {
                for element in &tuple.elts {
                    self.collect_assignment_target(owner, element, types);
                }
            }
            ast::Expr::List(list) => {
                for element in &list.elts {
                    self.collect_assignment_target(owner, element, types);
                }
            }
            _ => {}
        }
    }

    fn function_type_bindings(
        &self,
        function: &ast::StmtFunctionDef,
        class_name: Option<&str>,
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
                if let Some(type_name) =
                    type_binding_from_expr(self.module, self.imports, annotation)
                {
                    types.insert(parameter.name.as_str().to_string(), type_name);
                }
            }
        }
        types
    }

    fn collect_member_reference(
        &mut self,
        owner: &str,
        attribute: &ast::ExprAttribute,
        access: AccessKind,
        types: &HashMap<String, TypeBinding>,
    ) {
        let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
            return;
        };
        let receiver_name = receiver.id.as_str();
        if self.imports.iter().any(|import| {
            import.binding == receiver_name && matches!(import.target, ImportTarget::Module { .. })
        }) {
            return;
        }
        if let Some(receiver_type) = types.get(receiver_name) {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", receiver_type.base, attribute.attr.as_str()),
                access,
                attribute.range,
            );
        } else {
            self.unresolved_receivers.push(UnresolvedReceiver {
                from: owner.to_string(),
                receiver: receiver_name.to_string(),
                member: attribute.attr.as_str().to_string(),
                span: self
                    .locator
                    .span_from_range_string(self.file, attribute.range),
            });
        }
    }
}

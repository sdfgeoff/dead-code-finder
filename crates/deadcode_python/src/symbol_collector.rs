#[path = "symbol_expr.rs"]
mod symbol_expr;

use std::collections::{HashMap, HashSet};

use deadcode_core::SymbolKind;
use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use self::symbol_expr::{
    dotted_expr, first_module_segment, is_main_guard, self_attribute_name, target_name,
};
use super::{
    ImportTarget, IndexedSymbol, MemberReference, ResolvedImport, SourceLocator, SymbolReference,
    UnresolvedReceiver,
};

pub(super) struct SymbolCollector<'a> {
    pub(super) module: &'a str,
    pub(super) file: &'a str,
    pub(super) locator: &'a SourceLocator,
    pub(super) symbols: &'a mut Vec<IndexedSymbol>,
    pub(super) imports: &'a mut Vec<ResolvedImport>,
    pub(super) references: &'a mut Vec<SymbolReference>,
    pub(super) member_references: &'a mut Vec<MemberReference>,
    pub(super) unresolved_receivers: &'a mut Vec<UnresolvedReceiver>,
    pub(super) has_main_entrypoint: &'a mut bool,
    pub(super) known_modules: &'a HashSet<String>,
}

impl SymbolCollector<'_> {
    pub(super) fn collect_suite(&mut self, suite: &[ast::Stmt]) {
        for statement in suite {
            self.collect_module_statement(statement);
        }
    }

    fn collect_module_statement(&mut self, statement: &ast::Stmt) {
        match statement {
            ast::Stmt::FunctionDef(function) => {
                let function_owner = format!("{}.{}", self.module, function.name.as_str());
                self.push_symbol(
                    function_owner.clone(),
                    function.name.as_str(),
                    SymbolKind::Function,
                    function.range,
                );
                let types = self.function_type_bindings(function, None);
                self.collect_function_references(&function_owner, &function.body, types);
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
            ast::Stmt::If(if_stmt) if is_main_guard(if_stmt) => {
                *self.has_main_entrypoint = true;
                let mut types = HashMap::new();
                self.collect_statement_references(self.module, statement, &mut types);
            }
            statement => {
                let mut types = HashMap::new();
                self.collect_statement_references(self.module, statement, &mut types);
            }
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
                    let method_owner = format!("{}.{}.{}", self.module, class_name, method_name);
                    let types = self.function_type_bindings(function, Some(class_name));
                    self.collect_function_references(&method_owner, &function.body, types);
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

    fn push_reference(&mut self, from: &str, name: &str, range: TextRange) {
        self.references.push(SymbolReference {
            from: from.to_string(),
            name: name.to_string(),
            span: self.locator.span_from_range_string(self.file, range),
        });
    }

    fn collect_function_references(
        &mut self,
        owner: &str,
        body: &[ast::Stmt],
        mut types: HashMap<String, String>,
    ) {
        for statement in body {
            self.collect_statement_references(owner, statement, &mut types);
        }
    }

    fn collect_statement_references(
        &mut self,
        owner: &str,
        statement: &ast::Stmt,
        types: &mut HashMap<String, String>,
    ) {
        match statement {
            ast::Stmt::FunctionDef(function) => {
                let function_owner = format!("{}.{}", self.module, function.name.as_str());
                let types = self.function_type_bindings(function, None);
                self.collect_function_references(&function_owner, &function.body, types);
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
                if let Some(type_name) = self.constructor_type_name(&assign.value) {
                    for target in &assign.targets {
                        if let Some(name) = target_name(target) {
                            types.insert(name.to_string(), type_name.clone());
                        }
                    }
                }
            }
            ast::Stmt::AnnAssign(assign) => {
                if let Some(name) = target_name(&assign.target) {
                    if let Some(type_name) = self.type_name_from_expr(&assign.annotation) {
                        types.insert(name.to_string(), type_name);
                    }
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
        types: &HashMap<String, String>,
    ) {
        match expr {
            ast::Expr::Name(name) => self.push_reference(owner, name.id.as_str(), name.range),
            ast::Expr::Call(call) => {
                self.collect_expr_references(owner, &call.func, types);
                for arg in &call.arguments.args {
                    self.collect_expr_references(owner, arg, types);
                }
                for keyword in &call.arguments.keywords {
                    self.collect_expr_references(owner, &keyword.value, types);
                }
            }
            ast::Expr::Attribute(attribute) => {
                self.collect_member_reference(owner, attribute, types);
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

    fn function_type_bindings(
        &self,
        function: &ast::StmtFunctionDef,
        class_name: Option<&str>,
    ) -> HashMap<String, String> {
        let mut types = HashMap::new();
        if let Some(class_name) = class_name {
            types.insert(
                "self".to_string(),
                format!("{}.{}", self.module, class_name),
            );
            types.insert("cls".to_string(), format!("{}.{}", self.module, class_name));
        }
        for parameter in function.parameters.iter() {
            let parameter = parameter.as_parameter();
            if let Some(annotation) = parameter.annotation() {
                if let Some(type_name) = self.type_name_from_expr(annotation) {
                    types.insert(parameter.name.as_str().to_string(), type_name);
                }
            }
        }
        types
    }

    fn constructor_type_name(&self, expr: &ast::Expr) -> Option<String> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        self.type_name_from_expr(&call.func)
    }

    fn type_name_from_expr(&self, expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Name(name) => self.resolve_name_to_symbol(name.id.as_str()),
            ast::Expr::Attribute(attribute) => dotted_expr(attribute).and_then(|dotted| {
                self.imports.iter().find_map(|import| {
                    let ImportTarget::Module {
                        module,
                        external: false,
                    } = &import.target
                    else {
                        return None;
                    };
                    dotted
                        .strip_prefix(&import.binding)
                        .and_then(|suffix| suffix.strip_prefix('.'))
                        .map(|suffix| format!("{module}.{suffix}"))
                })
            }),
            ast::Expr::Subscript(subscript) => self.type_name_from_expr(&subscript.value),
            _ => None,
        }
    }

    fn resolve_name_to_symbol(&self, name: &str) -> Option<String> {
        for import in self.imports.iter() {
            if import.binding != name {
                continue;
            }
            return match &import.target {
                ImportTarget::Symbol {
                    module,
                    name,
                    external: false,
                } => Some(format!("{module}.{name}")),
                ImportTarget::Module {
                    module,
                    external: false,
                } => Some(module.clone()),
                _ => None,
            };
        }
        Some(format!("{}.{}", self.module, name))
    }

    fn collect_member_reference(
        &mut self,
        owner: &str,
        attribute: &ast::ExprAttribute,
        types: &HashMap<String, String>,
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
            self.member_references.push(MemberReference {
                from: owner.to_string(),
                target: format!("{}.{}", receiver_type, attribute.attr.as_str()),
                span: self
                    .locator
                    .span_from_range_string(self.file, attribute.range),
            });
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

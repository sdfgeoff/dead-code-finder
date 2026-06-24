use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_construction::constructed_type_for_call;
use super::symbol_expr::target_name;
use super::symbol_imports::{collect_import, collect_import_from};
use super::symbol_iteration::{bind_collection_unpack_target, bind_iteration_target};
use super::symbol_members::push_member_reference;
use super::symbol_rules::{callable_argument_references, callable_identity, constructor_binding};
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::{
    AccessKind, CallArgumentType, ImportTarget, TypeBinding, UnsupportedExpansion,
};

impl SymbolCollector<'_> {
    pub(super) fn collect_statement_references(
        &mut self,
        owner: &str,
        statement: &ast::Stmt,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        match statement {
            ast::Stmt::FunctionDef(function) => {
                let function_owner = format!("{}.{}", self.module, function.name.as_str());
                let types = self.function_type_bindings(function, None, types);
                self.collect_function_references(&function_owner, function, types);
            }
            ast::Stmt::ClassDef(_) => {}
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
                self.push_imported_value_bindings(types, import_start);
            }
            ast::Stmt::Expr(expr) => {
                self.bind_append_receiver_type(&expr.value, types);
                self.collect_expr_references(owner, &expr.value, types);
            }
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
                if let Some(mut type_name) = self.assignment_value_binding(&assign.value, types) {
                    self.mark_external_if_outside_project(&mut type_name);
                    for target in &assign.targets {
                        if let Some(name) = target_name(target) {
                            types.insert(name.to_string(), type_name.clone());
                            if owner == self.module {
                                self.push_value_binding(name, type_name.clone());
                            }
                        } else {
                            bind_collection_unpack_target(target, &type_name, types);
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
            ast::Stmt::AugAssign(assign) => {
                self.collect_assignment_target(owner, &assign.target, types);
                self.collect_expr_references(owner, &assign.value, types);
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
            ast::Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    self.collect_expr_references(owner, &item.context_expr, types);
                    self.collect_context_manager_references(owner, &item.context_expr, types);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.collect_assignment_target(owner, optional_vars, types);
                        self.bind_context_manager_optional_var(
                            optional_vars,
                            &item.context_expr,
                            types,
                        );
                    }
                }
                for nested in &with_stmt.body {
                    self.collect_statement_references(owner, nested, types);
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.collect_expr_references(owner, &for_stmt.iter, types);
                self.collect_assignment_target(owner, &for_stmt.target, types);
                let item_type = self.iteration_item_type(&for_stmt.iter, types);
                if let (Some(name), Some(item_type)) = (target_name(&for_stmt.target), item_type) {
                    types.insert(name.to_string(), item_type);
                } else if let Some(item_type) = self.iteration_item_type(&for_stmt.iter, types) {
                    bind_iteration_target(&for_stmt.target, &item_type, types);
                }
                for nested in &for_stmt.body {
                    self.collect_statement_references(owner, nested, types);
                }
                for nested in &for_stmt.orelse {
                    self.collect_statement_references(owner, nested, types);
                }
            }
            ast::Stmt::While(while_stmt) => {
                self.collect_expr_references(owner, &while_stmt.test, types);
                for nested in &while_stmt.body {
                    self.collect_statement_references(owner, nested, types);
                }
                for nested in &while_stmt.orelse {
                    self.collect_statement_references(owner, nested, types);
                }
            }
            ast::Stmt::Try(try_stmt) => {
                for nested in &try_stmt.body {
                    self.collect_statement_references(owner, nested, types);
                }
                for handler in &try_stmt.handlers {
                    match handler {
                        ast::ExceptHandler::ExceptHandler(handler) => {
                            let mut handler_types = types.clone();
                            if let Some(type_) = &handler.type_ {
                                self.collect_expr_references(owner, type_, types);
                                if let (Some(name), Some(binding)) = (
                                    handler.name.as_ref(),
                                    type_binding_from_expr(self.module, self.imports, type_),
                                ) {
                                    handler_types.insert(name.as_str().to_string(), binding);
                                }
                            }
                            for nested in &handler.body {
                                self.collect_statement_references(
                                    owner,
                                    nested,
                                    &mut handler_types,
                                );
                            }
                        }
                    }
                }
                for nested in &try_stmt.orelse {
                    self.collect_statement_references(owner, nested, types);
                }
                for nested in &try_stmt.finalbody {
                    self.collect_statement_references(owner, nested, types);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_expr_references(
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
            ast::Expr::Subscript(subscript) => {
                self.collect_expr_references(owner, &subscript.value, types);
                self.collect_expr_references(owner, &subscript.slice, types);
            }
            ast::Expr::Await(await_expr) => {
                self.collect_expr_references(owner, &await_expr.value, types);
            }
            ast::Expr::If(if_expr) => {
                self.collect_expr_references(owner, &if_expr.test, types);
                self.collect_expr_references(owner, &if_expr.body, types);
                self.collect_expr_references(owner, &if_expr.orelse, types);
            }
            ast::Expr::Tuple(tuple) => {
                for element in &tuple.elts {
                    self.collect_expr_references(owner, element, types);
                }
            }
            ast::Expr::List(list) => {
                for element in &list.elts {
                    self.collect_expr_references(owner, element, types);
                }
            }
            ast::Expr::ListComp(list_comp) => {
                self.collect_list_comprehension_references(owner, list_comp, types);
            }
            ast::Expr::Set(set) => {
                for element in &set.elts {
                    self.collect_expr_references(owner, element, types);
                }
            }
            ast::Expr::Dict(dict) => {
                for item in &dict.items {
                    if let Some(key) = &item.key {
                        self.collect_expr_references(owner, key, types);
                    }
                    self.collect_expr_references(owner, &item.value, types);
                }
            }
            ast::Expr::FString(f_string) => self.collect_fstring_references(owner, f_string, types),
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
                constructor_binding(self.module, self.imports, self.rules, arg)
                    .or_else(|| self.class_object_argument_binding(arg)),
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
        let constructor =
            constructed_type_for_call(self.module, self.imports, self.rules, &call.func, types);
        if let Some((constructor_type, _)) = constructor.as_ref() {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{constructor_type}.__init__"),
                AccessKind::Construct,
                call.func.range(),
            );
        }
        for keyword in &call.arguments.keywords {
            if self.collect_max_key_lambda_references(owner, call, keyword, types) {
                continue;
            }
            self.collect_expr_references(owner, &keyword.value, types);
            let Some((constructor_type, is_type_parameter)) = constructor.as_ref() else {
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
            } else if self.expand_model_dump_keyword(
                owner,
                constructor_type,
                &keyword.value,
                types,
                keyword.range,
            ) {
                continue;
            } else if !is_type_parameter {
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

    fn class_object_argument_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        match expr {
            ast::Expr::Name(name) => self.class_object_binding(name.id.as_str()),
            _ => None,
        }
    }

    fn bind_append_receiver_type(
        &self,
        expr: &ast::Expr,
        types: &mut HashMap<String, TypeBinding>,
    ) -> Option<()> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        if attribute.attr.as_str() != "append" {
            return None;
        }
        let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
            return None;
        };
        if types.contains_key(receiver.id.as_str()) {
            return None;
        }
        let item_type = call
            .arguments
            .args
            .first()
            .and_then(|arg| self.assignment_value_binding(arg, types))?;
        types.insert(
            receiver.id.as_str().to_string(),
            TypeBinding {
                base: "list".to_string(),
                args: vec![item_type],
                external: false,
            },
        );
        Some(())
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

    pub(super) fn class_object_binding(&self, receiver_name: &str) -> Option<TypeBinding> {
        for import in self.imports.iter() {
            if import.binding != receiver_name {
                continue;
            }
            return match &import.target {
                ImportTarget::Module {
                    module,
                    external: false,
                } => Some(TypeBinding::erased(module.clone())),
                ImportTarget::Symbol {
                    module,
                    name,
                    external: false,
                } => Some(TypeBinding::erased(format!("{module}.{name}"))),
                ImportTarget::Symbol { external: true, .. } => return None,
                _ => None,
            };
        }
        let same_module = format!("{}.{}", self.module, receiver_name);
        self.available_classes
            .iter()
            .any(|class_info| class_info.class == same_module)
            .then(|| TypeBinding::erased(same_module))
    }

    pub(super) fn push_imported_value_bindings(
        &mut self,
        types: &mut HashMap<String, TypeBinding>,
        import_start: usize,
    ) {
        let imports = &self.imports[import_start..];
        for import in imports {
            let Some(qualified_name) = import_value_qualified_name(&import.target) else {
                continue;
            };
            let Some(binding) = self
                .available_values
                .iter()
                .find(|value| value.qualified_name == qualified_name)
                .map(|value| value.binding.clone())
            else {
                continue;
            };
            types.insert(import.binding.clone(), binding);
        }
    }

    fn push_value_binding(&mut self, name: &str, binding: TypeBinding) {
        let qualified_name = format!("{}.{}", self.module, name);
        if let Some(existing) = self
            .value_bindings
            .iter_mut()
            .find(|value| value.qualified_name == qualified_name)
        {
            existing.binding = binding;
            return;
        }
        self.value_bindings.push(crate::symbol_index::ValueBinding {
            qualified_name,
            binding,
        });
    }
}

fn import_value_qualified_name(target: &ImportTarget) -> Option<String> {
    match target {
        ImportTarget::Symbol {
            module,
            name,
            external: false,
        } => Some(format!("{module}.{name}")),
        _ => None,
    }
}

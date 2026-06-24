use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_expr::target_name;
use super::symbol_generics::field_read_type;
use super::symbol_members::push_member_reference;
use super::symbol_rules::{
    callable_argument_references, callable_identity, constructed_type_from_callee,
    constructor_binding,
};
use super::symbol_types::{type_binding_from_expr, TypeBinding};
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, CallArgumentType, ImportTarget, UnsupportedExpansion};

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
            ast::Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    self.collect_expr_references(owner, &item.context_expr, types);
                    self.collect_context_manager_references(owner, &item.context_expr, types);
                    if let Some(optional_vars) = &item.optional_vars {
                        self.collect_assignment_target(owner, optional_vars, types);
                    }
                }
                for nested in &with_stmt.body {
                    self.collect_statement_references(owner, nested, types);
                }
            }
            ast::Stmt::For(for_stmt) => {
                self.collect_expr_references(owner, &for_stmt.iter, types);
                self.collect_assignment_target(owner, &for_stmt.target, types);
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
                            if let Some(type_) = &handler.type_ {
                                self.collect_expr_references(owner, type_, types);
                            }
                            for nested in &handler.body {
                                self.collect_statement_references(owner, nested, types);
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
        if let Some(constructor_type) = constructor_type.as_ref() {
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

    fn collect_context_manager_references(
        &mut self,
        owner: &str,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        let Some(binding) = constructor_binding(self.module, self.imports, self.rules, expr)
            .or_else(|| field_read_type(self.classes, expr, types))
        else {
            return;
        };
        for method in ["__enter__", "__exit__"] {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", binding.base, method),
                AccessKind::Call,
                expr.range(),
            );
        }
    }

    fn collect_member_reference(
        &mut self,
        owner: &str,
        attribute: &ast::ExprAttribute,
        access: AccessKind,
        types: &HashMap<String, TypeBinding>,
    ) {
        let receiver_type = match attribute.value.as_ref() {
            ast::Expr::Name(receiver) => {
                let receiver_name = receiver.id.as_str();
                if self.imports.iter().any(|import| {
                    import.binding == receiver_name && import_target_is_external(&import.target)
                }) {
                    return;
                }
                types
                    .get(receiver_name)
                    .cloned()
                    .or_else(|| self.class_object_binding(receiver_name))
            }
            value => field_read_type(self.classes, value, types),
        };
        if let Some(receiver_type) = receiver_type {
            if receiver_type.external {
                return;
            }
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
            let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
                return;
            };
            self.unresolved_receivers
                .push(crate::symbol_index::UnresolvedReceiver {
                    from: owner.to_string(),
                    receiver: receiver.id.as_str().to_string(),
                    member: attribute.attr.as_str().to_string(),
                    span: self
                        .locator
                        .span_from_range_string(self.file, attribute.range),
                });
        }
    }

    fn class_object_binding(&self, receiver_name: &str) -> Option<TypeBinding> {
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
        self.classes
            .iter()
            .any(|class_info| class_info.class == same_module)
            .then(|| TypeBinding::erased(same_module))
    }
}

fn import_target_is_external(target: &ImportTarget) -> bool {
    match target {
        ImportTarget::Module { external, .. }
        | ImportTarget::Symbol { external, .. }
        | ImportTarget::Star { external, .. } => *external,
    }
}

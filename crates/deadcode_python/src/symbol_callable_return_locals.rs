use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use super::symbol_rules::local_callable_identity;
use super::SymbolCollector;
use crate::symbol_index::CallableReturnMemberUse;

impl SymbolCollector<'_> {
    pub(super) fn collect_callable_return_local_member_uses(
        &mut self,
        owner: &str,
        suite: &[ast::Stmt],
    ) {
        let mut origins = HashMap::new();
        self.collect_callable_return_local_suite(owner, suite, &mut origins);
    }

    fn collect_callable_return_local_suite(
        &mut self,
        owner: &str,
        suite: &[ast::Stmt],
        origins: &mut HashMap<String, String>,
    ) {
        for statement in suite {
            self.collect_callable_return_local_statement(owner, statement, origins);
        }
    }

    fn collect_callable_return_local_statement(
        &mut self,
        owner: &str,
        statement: &ast::Stmt,
        origins: &mut HashMap<String, String>,
    ) {
        match statement {
            ast::Stmt::Assign(assign) => {
                self.collect_callable_return_local_expr(owner, &assign.value, origins);
                if assign.targets.len() == 1 {
                    if let (Some(target), Some(callable)) = (
                        assign.targets.first().and_then(target_name),
                        callable_origin(self.module, &assign.value),
                    ) {
                        origins.insert(target.to_string(), callable);
                        return;
                    }
                }
                for target in &assign.targets {
                    if let Some(target) = target_name(target) {
                        origins.remove(target);
                    }
                }
            }
            ast::Stmt::AnnAssign(assign) => {
                if let Some(value) = &assign.value {
                    self.collect_callable_return_local_expr(owner, value, origins);
                    if let (Some(target), Some(callable)) = (
                        target_name(&assign.target),
                        callable_origin(self.module, value),
                    ) {
                        origins.insert(target.to_string(), callable);
                        return;
                    }
                }
                if let Some(target) = target_name(&assign.target) {
                    origins.remove(target);
                }
            }
            ast::Stmt::Expr(expr) => {
                self.collect_callable_return_local_expr(owner, &expr.value, origins);
            }
            ast::Stmt::Return(ret) => {
                if let Some(value) = &ret.value {
                    self.collect_callable_return_local_expr(owner, value, origins);
                }
            }
            ast::Stmt::If(if_stmt) => {
                self.collect_callable_return_local_expr(owner, &if_stmt.test, origins);
                let mut body_origins = origins.clone();
                self.collect_callable_return_local_suite(owner, &if_stmt.body, &mut body_origins);
                for clause in &if_stmt.elif_else_clauses {
                    let mut clause_origins = origins.clone();
                    if let Some(test) = &clause.test {
                        self.collect_callable_return_local_expr(owner, test, origins);
                    }
                    self.collect_callable_return_local_suite(
                        owner,
                        &clause.body,
                        &mut clause_origins,
                    );
                }
            }
            ast::Stmt::With(with_stmt) => {
                for item in &with_stmt.items {
                    self.collect_callable_return_local_expr(owner, &item.context_expr, origins);
                }
                self.collect_callable_return_local_suite(owner, &with_stmt.body, origins);
            }
            ast::Stmt::For(for_stmt) => {
                self.collect_callable_return_local_expr(owner, &for_stmt.iter, origins);
                if let Some(target) = target_name(&for_stmt.target) {
                    origins.remove(target);
                }
                self.collect_callable_return_local_suite(owner, &for_stmt.body, origins);
                self.collect_callable_return_local_suite(owner, &for_stmt.orelse, origins);
            }
            ast::Stmt::While(while_stmt) => {
                self.collect_callable_return_local_expr(owner, &while_stmt.test, origins);
                self.collect_callable_return_local_suite(owner, &while_stmt.body, origins);
                self.collect_callable_return_local_suite(owner, &while_stmt.orelse, origins);
            }
            ast::Stmt::Try(try_stmt) => {
                self.collect_callable_return_local_suite(owner, &try_stmt.body, origins);
                for handler in &try_stmt.handlers {
                    let ast::ExceptHandler::ExceptHandler(handler) = handler;
                    self.collect_callable_return_local_suite(owner, &handler.body, origins);
                }
                self.collect_callable_return_local_suite(owner, &try_stmt.orelse, origins);
                self.collect_callable_return_local_suite(owner, &try_stmt.finalbody, origins);
            }
            ast::Stmt::Assert(assert_stmt) => {
                self.collect_callable_return_local_expr(owner, &assert_stmt.test, origins);
                if let Some(msg) = &assert_stmt.msg {
                    self.collect_callable_return_local_expr(owner, msg, origins);
                }
            }
            ast::Stmt::Raise(raise_stmt) => {
                if let Some(exc) = &raise_stmt.exc {
                    self.collect_callable_return_local_expr(owner, exc, origins);
                }
                if let Some(cause) = &raise_stmt.cause {
                    self.collect_callable_return_local_expr(owner, cause, origins);
                }
            }
            _ => {}
        }
    }

    fn collect_callable_return_local_expr(
        &mut self,
        owner: &str,
        expr: &ast::Expr,
        origins: &HashMap<String, String>,
    ) {
        match expr {
            ast::Expr::Call(call) => {
                if let ast::Expr::Attribute(attribute) = call.func.as_ref() {
                    if let ast::Expr::Name(receiver) = attribute.value.as_ref() {
                        if let Some(callable) = origins.get(receiver.id.as_str()) {
                            self.callable_return_member_uses
                                .push(CallableReturnMemberUse {
                                    from: owner.to_string(),
                                    callable: callable.clone(),
                                    member: attribute.attr.as_str().to_string(),
                                    span: self
                                        .locator
                                        .span_from_range_string(self.file, attribute.range),
                                });
                        }
                    }
                }
                self.collect_callable_return_local_expr(owner, &call.func, origins);
                for argument in &call.arguments.args {
                    self.collect_callable_return_local_expr(owner, argument, origins);
                }
                for keyword in &call.arguments.keywords {
                    self.collect_callable_return_local_expr(owner, &keyword.value, origins);
                }
            }
            ast::Expr::Attribute(attribute) => {
                self.collect_callable_return_local_expr(owner, &attribute.value, origins);
            }
            ast::Expr::BoolOp(bool_op) => {
                for value in &bool_op.values {
                    self.collect_callable_return_local_expr(owner, value, origins);
                }
            }
            ast::Expr::BinOp(bin_op) => {
                self.collect_callable_return_local_expr(owner, &bin_op.left, origins);
                self.collect_callable_return_local_expr(owner, &bin_op.right, origins);
            }
            ast::Expr::UnaryOp(unary_op) => {
                self.collect_callable_return_local_expr(owner, &unary_op.operand, origins);
            }
            ast::Expr::If(if_expr) => {
                self.collect_callable_return_local_expr(owner, &if_expr.test, origins);
                self.collect_callable_return_local_expr(owner, &if_expr.body, origins);
                self.collect_callable_return_local_expr(owner, &if_expr.orelse, origins);
            }
            ast::Expr::List(list) => {
                for element in &list.elts {
                    self.collect_callable_return_local_expr(owner, element, origins);
                }
            }
            ast::Expr::Tuple(tuple) => {
                for element in &tuple.elts {
                    self.collect_callable_return_local_expr(owner, element, origins);
                }
            }
            ast::Expr::Dict(dict) => {
                for item in &dict.items {
                    if let Some(key) = &item.key {
                        self.collect_callable_return_local_expr(owner, key, origins);
                    }
                    self.collect_callable_return_local_expr(owner, &item.value, origins);
                }
            }
            ast::Expr::Subscript(subscript) => {
                self.collect_callable_return_local_expr(owner, &subscript.value, origins);
                self.collect_callable_return_local_expr(owner, &subscript.slice, origins);
            }
            ast::Expr::Await(await_expr) => {
                self.collect_callable_return_local_expr(owner, &await_expr.value, origins);
            }
            _ => {}
        }
    }
}

fn callable_origin(module: &str, expr: &ast::Expr) -> Option<String> {
    let ast::Expr::Call(call) = expr else {
        return None;
    };
    local_callable_identity(module, &call.func)
}

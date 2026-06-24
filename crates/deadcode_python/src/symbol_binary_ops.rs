use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_datetime::{is_datetime_like, is_timedelta};
use super::symbol_rules::callable_identity;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn binop_expression_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::BinOp(bin_op) = expr else {
            return None;
        };
        if bin_op.op == ast::Operator::Div && self.is_pathlib_path_expr(&bin_op.left) {
            return Some(TypeBinding {
                base: "pathlib.Path".to_string(),
                args: Vec::new(),
                external: true,
            });
        }
        let left = self.expression_flow_binding(&bin_op.left, types)?;
        let right = self.expression_flow_binding(&bin_op.right, types)?;
        match bin_op.op {
            ast::Operator::Add if same_list_type(&left, &right) => Some(left),
            ast::Operator::Add if is_datetime_like(&left) && is_timedelta(&right) => Some(left),
            ast::Operator::Sub if is_datetime_like(&left) && is_timedelta(&right) => Some(left),
            ast::Operator::Div if left.external => Some(left),
            _ => None,
        }
    }

    fn is_pathlib_path_expr(&self, expr: &ast::Expr) -> bool {
        match expr {
            ast::Expr::Call(call) => match call.func.as_ref() {
                ast::Expr::Name(_) => {
                    callable_identity(self.module, self.imports, &call.func).as_deref()
                        == Some("pathlib.Path")
                }
                ast::Expr::Attribute(attribute) => self.is_pathlib_path_expr(&attribute.value),
                _ => false,
            },
            ast::Expr::Attribute(attribute) => self.is_pathlib_path_expr(&attribute.value),
            ast::Expr::BinOp(bin_op) if bin_op.op == ast::Operator::Div => {
                self.is_pathlib_path_expr(&bin_op.left)
            }
            _ => false,
        }
    }
}

fn same_list_type(left: &TypeBinding, right: &TypeBinding) -> bool {
    is_list_type(&left.base)
        && is_list_type(&right.base)
        && left.args == right.args
        && left.external == right.external
}

fn is_list_type(type_name: &str) -> bool {
    matches!(type_name, "list" | "typing.List" | "List") || type_name.ends_with(".list")
}

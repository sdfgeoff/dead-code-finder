use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_expr::target_name;
use super::symbol_generics::{
    collection_item_type, member_reference_target_bases, substitute_type_params,
};
use super::symbol_iteration::bind_iteration_target;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_with_statement_references(
        &mut self,
        owner: &str,
        with_stmt: &ast::StmtWith,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        for item in &with_stmt.items {
            self.collect_expr_references(owner, &item.context_expr, types);
            self.collect_context_manager_references(owner, &item.context_expr, types);
            if let Some(optional_vars) = &item.optional_vars {
                self.collect_assignment_target(owner, optional_vars, types);
                self.bind_context_manager_optional_var(optional_vars, &item.context_expr, types);
            }
        }
        for nested in &with_stmt.body {
            self.collect_statement_references(owner, nested, types);
        }
    }

    pub(super) fn collect_for_statement_references(
        &mut self,
        owner: &str,
        for_stmt: &ast::StmtFor,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        self.collect_enum_iteration_references(owner, &for_stmt.iter);
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

    pub(super) fn subscript_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Subscript(subscript) = expr else {
            return None;
        };
        let collection_type = self.expression_flow_binding(&subscript.value, types)?;
        if matches!(subscript.slice.as_ref(), ast::Expr::Slice(_)) {
            Some(collection_type)
        } else {
            collection_item_type(&collection_type)
                .or_else(|| self.getitem_return_binding(&collection_type))
        }
    }

    fn getitem_return_binding(&self, receiver_type: &TypeBinding) -> Option<TypeBinding> {
        let returns = member_reference_target_bases(receiver_type)
            .into_iter()
            .filter_map(|base| self.method_return_binding(&base, "__getitem__", receiver_type))
            .collect::<Vec<_>>();
        match returns.as_slice() {
            [] => None,
            [binding] => Some(binding.clone()),
            _ => Some(TypeBinding {
                base: "typing.Union".to_string(),
                args: returns,
                external: false,
            }),
        }
    }

    fn method_return_binding(
        &self,
        class_name: &str,
        method_name: &str,
        receiver_type: &TypeBinding,
    ) -> Option<TypeBinding> {
        let method = format!("{class_name}.{method_name}");
        let signature = self
            .available_fn_sigs
            .iter()
            .find(|signature| signature.function == method)?;
        let return_type = signature
            .concrete_return_type
            .clone()
            .or_else(|| signature.return_type.clone())?;
        let class_info = self
            .available_classes
            .iter()
            .find(|class_info| class_info.class == class_name)?;
        Some(expand_alias_binding(
            &substitute_type_params(&return_type, class_info, receiver_type),
            self.available_values,
        ))
    }
}

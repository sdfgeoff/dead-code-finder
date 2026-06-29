use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_expr::target_name;
use super::symbol_iteration::bind_collection_unpack_target;
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_assign_references(
        &mut self,
        owner: &str,
        assign: &ast::StmtAssign,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        let validated_type = self.validated_assignment_binding(&assign.value, types);
        self.collect_expr_references(owner, &assign.value, types);
        if owner == self.module {
            for target in &assign.targets {
                if let Some(name) = target_name(target) {
                    self.collect_module_value_initializer(name, &assign.value, types);
                }
            }
        }
        for target in &assign.targets {
            self.collect_assignment_target(owner, target, types);
        }
        if let Some(mut type_name) = self.assignment_value_binding(&assign.value, types) {
            self.mark_external_if_outside_project(&mut type_name);
            for target in &assign.targets {
                if let Some(name) = target_name(target) {
                    types.insert(name.to_string(), type_name.clone());
                    self.bind_validated_assignment(target, validated_type.as_ref(), types);
                    if owner == self.module {
                        self.push_value_binding(name, type_name.clone());
                    }
                } else {
                    bind_collection_unpack_target(target, &type_name, types);
                }
            }
        }
    }

    pub(super) fn collect_ann_assign_references(
        &mut self,
        owner: &str,
        assign: &ast::StmtAnnAssign,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        if let Some(name) = target_name(&assign.target) {
            if let Some(mut type_name) =
                type_binding_from_expr(self.module, self.imports, &assign.annotation)
            {
                type_name = expand_alias_binding(&type_name, self.available_values);
                types.insert(name.to_string(), type_name.clone());
                if owner == self.module {
                    self.push_value_binding(name, type_name);
                }
            }
        } else {
            self.collect_assignment_target(owner, &assign.target, types);
        }
        if let Some(value) = &assign.value {
            if owner == self.module {
                if let Some(name) = target_name(&assign.target) {
                    self.collect_module_value_initializer(name, value, types);
                }
            }
            if let Some(expected_type) =
                type_binding_from_expr(self.module, self.imports, &assign.annotation)
            {
                let expected_type = expand_alias_binding(&expected_type, self.available_values);
                self.collect_typed_dict_literal_construction(owner, &expected_type, value);
            }
            let validated_type = self.validated_assignment_binding(value, types);
            self.collect_expr_references(owner, value, types);
            self.bind_validated_assignment(&assign.target, validated_type.as_ref(), types);
        }
    }

    fn collect_module_value_initializer(
        &mut self,
        name: &str,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        let value_owner = format!("{}.{}", self.module, name);
        self.push_module_value(name);
        self.collect_expr_references(&value_owner, value, types);
    }
}

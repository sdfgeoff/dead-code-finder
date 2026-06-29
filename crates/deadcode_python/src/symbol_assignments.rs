use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_expr::target_name;
use super::symbol_iteration::bind_collection_unpack_target;
use super::symbol_rules::callable_identity;
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::{DependencyOverride, TypeBinding};

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
            self.collect_dependency_override(owner, target, &assign.value, types);
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

    fn collect_dependency_override(
        &mut self,
        owner: &str,
        target: &ast::Expr,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        let Some((dependency, target_range)) = self.dependency_override_target(target, types)
        else {
            return;
        };
        let Some(concrete_type) = self.override_return_binding(value, types) else {
            return;
        };
        self.dependency_overrides.push(DependencyOverride {
            from: owner.to_string(),
            dependency,
            concrete_type: concrete_type.base,
            span: self.locator.span_from_range_string(self.file, target_range),
        });
    }

    fn dependency_override_target(
        &self,
        target: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<(String, ruff_text_size::TextRange)> {
        let ast::Expr::Subscript(subscript) = target else {
            return None;
        };
        let ast::Expr::Attribute(attribute) = subscript.value.as_ref() else {
            return None;
        };
        for rule in &self.rules.assignments {
            if rule.effect != "overrideCallableReturn" || attribute.attr.as_str() != rule.member {
                continue;
            }
            let receiver_type = self.receiver_type_for_expr(&attribute.value, types)?;
            if receiver_type.base != rule.receiver_type {
                continue;
            }
            let dependency = callable_identity(self.module, self.imports, &subscript.slice)?;
            return Some((dependency, subscript.slice.range()));
        }
        None
    }

    pub(super) fn override_return_binding(
        &self,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        if let ast::Expr::Lambda(lambda) = value {
            return self.expression_or_name_binding(&lambda.body, types);
        }
        if let Some(binding) = self.callable_factory_return_binding(value, types) {
            return Some(binding);
        }
        self.expression_or_name_binding(value, types)
    }

    fn callable_factory_return_binding(
        &self,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = value else {
            return None;
        };
        let binding = self.assignment_value_binding(value, types)?;
        if !is_callable_type(&binding.base) {
            return None;
        }
        let return_type = binding.args.last()?;
        for argument in &call.arguments.args {
            for concrete_type in self.override_argument_concrete_types(argument, types) {
                if self.is_subclass_or_same(&concrete_type, &return_type.base)
                    || self.is_protocol_type(&return_type.base)
                {
                    return Some(TypeBinding::erased(concrete_type));
                }
            }
        }
        None
    }

    fn override_argument_concrete_types(
        &self,
        argument: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Vec<String> {
        if let ast::Expr::Name(name) = argument {
            if let Some(binding) = types.get(name.id.as_str()) {
                return concrete_types_from_binding(binding);
            }
        }
        self.concrete_argument_types(argument, types)
    }

    fn expression_or_name_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        if let ast::Expr::Name(name) = expr {
            if let Some(binding) = types.get(name.id.as_str()) {
                if let Some(return_type) = self.function_return_binding(&binding.base) {
                    return Some(return_type);
                }
                return Some(binding.clone());
            }
            let callable = callable_identity(self.module, self.imports, expr)?;
            return self.function_return_binding(&callable);
        }
        self.assignment_value_binding(expr, types)
    }

    fn function_return_binding(&self, function: &str) -> Option<TypeBinding> {
        self.available_fn_sigs
            .iter()
            .chain(self.fn_sigs.iter())
            .find(|signature| signature.function == function)
            .and_then(|signature| {
                signature
                    .concrete_return_type
                    .clone()
                    .or_else(|| signature.return_type.clone())
            })
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

    fn is_protocol_type(&self, type_name: &str) -> bool {
        self.available_classes
            .iter()
            .chain(self.classes.iter())
            .find(|class_info| class_info.class == type_name)
            .is_some_and(|class_info| {
                class_info
                    .bases
                    .iter()
                    .any(|base| matches!(base.base.as_str(), "typing.Protocol" | "Protocol"))
            })
    }
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    )
}

fn concrete_types_from_binding(binding: &TypeBinding) -> Vec<String> {
    if matches!(binding.base.as_str(), "typing.Union" | "types.UnionType") {
        return binding
            .args
            .iter()
            .flat_map(concrete_types_from_binding)
            .collect();
    }
    if is_callable_type(&binding.base) {
        return binding
            .args
            .last()
            .map(concrete_types_from_binding)
            .unwrap_or_default();
    }
    vec![binding.base.clone()]
}

use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_expr::target_name;
use super::symbol_typevars::substitute_type_vars;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn record_validated_return_from_expr(
        &mut self,
        owner: &str,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        self.record_pydantic_validated_return_type(owner, value, types);
        if let ast::Expr::Name(name) = value {
            if let Some(binding) = types.get(&validated_type_key(name.id.as_str())) {
                self.record_pydantic_validated_return_binding(owner, binding.clone());
            }
        }
    }

    pub(super) fn validated_assignment_binding(
        &self,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        self.pydantic_validation_call_binding(value, types)
    }

    pub(super) fn bind_validated_assignment(
        &self,
        target: &ast::Expr,
        validated_type: Option<&TypeBinding>,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        if let Some(name) = target_name(target) {
            if let Some(validated_type) = validated_type {
                types.insert(validated_type_key(name), validated_type.clone());
            } else {
                types.remove(&validated_type_key(name));
            }
        }
    }

    pub(super) fn local_call_validated_return_bindings(
        &self,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) -> Vec<TypeBinding> {
        let Some(callee) = self.resolved_call_target(&call.func, types) else {
            return Vec::new();
        };
        let Some(signature) = self
            .available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)
        else {
            return Vec::new();
        };
        let substitutions = self.type_var_substitutions(signature, call, types);
        signature
            .validated_return_types
            .iter()
            .map(|binding| {
                expand_alias_binding(
                    &substitute_type_vars(binding, &substitutions),
                    self.available_values,
                )
            })
            .collect()
    }
}

fn validated_type_key(name: &str) -> String {
    format!("$validated:{name}")
}

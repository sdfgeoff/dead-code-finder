use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_expr::target_name;
use super::symbol_generics::expr_type;
use super::symbol_members::push_member_reference;
use super::symbol_rules::{callable_identity, constructor_binding};
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn bind_context_manager_optional_var(
        &self,
        optional_vars: &ast::Expr,
        context_expr: &ast::Expr,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        let (Some(name), Some(binding)) = (
            target_name(optional_vars),
            self.contextmanager_generator_binding(context_expr)
                .or_else(|| {
                    constructor_binding(self.module, self.imports, self.rules, context_expr)
                })
                .or_else(|| expr_type(self.available_classes, context_expr, types)),
        ) else {
            return;
        };
        types.insert(name.to_string(), binding);
    }

    pub(super) fn collect_context_manager_references(
        &mut self,
        owner: &str,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) {
        let Some(binding) = constructor_binding(self.module, self.imports, self.rules, expr)
            .or_else(|| expr_type(self.available_classes, expr, types))
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

    fn contextmanager_generator_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        let ast::Expr::Call(context_call) = expr else {
            return None;
        };
        let ast::Expr::Call(wrapper_call) = context_call.func.as_ref() else {
            return None;
        };
        if callable_identity(self.module, self.imports, &wrapper_call.func).as_deref()
            != Some("contextlib.contextmanager")
        {
            return None;
        }
        let wrapped = wrapper_call.arguments.args.first()?;
        let wrapped_function = callable_identity(self.module, self.imports, wrapped)?;
        let return_type = self
            .available_fn_sigs
            .iter()
            .find(|signature| signature.function == wrapped_function)?
            .return_type
            .as_ref()?;
        generator_yield_type(return_type).cloned()
    }
}

fn generator_yield_type(binding: &TypeBinding) -> Option<&TypeBinding> {
    if !matches!(
        binding.base.as_str(),
        "typing.Generator" | "collections.abc.Generator" | "Generator"
    ) && !binding.base.ends_with(".Generator")
    {
        return None;
    }
    binding.args.first()
}

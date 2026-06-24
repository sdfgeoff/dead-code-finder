use ruff_python_ast as ast;

use super::symbol_rules::callable_identity;
use super::symbol_types::type_binding_from_expr;
use crate::symbol_index::{ResolvedImport, TypeBinding};

pub(super) fn type_alias_type_binding(
    module: &str,
    imports: &[ResolvedImport],
    value: &ast::Expr,
) -> Option<TypeBinding> {
    let ast::Expr::Call(call) = value else {
        return None;
    };
    if !is_type_alias_type(&callable_identity(module, imports, &call.func)?) {
        return None;
    }
    call.arguments
        .args
        .get(1)
        .and_then(|arg| type_binding_from_expr(module, imports, arg))
        .map(unwrap_annotated_alias)
}

fn is_type_alias_type(callable: &str) -> bool {
    matches!(
        callable,
        "typing_extensions.TypeAliasType" | "typing.TypeAliasType"
    )
}

fn unwrap_annotated_alias(binding: TypeBinding) -> TypeBinding {
    if matches!(
        binding.base.as_str(),
        "typing.Annotated" | "typing_extensions.Annotated"
    ) {
        return binding
            .args
            .into_iter()
            .next()
            .unwrap_or_else(|| TypeBinding::erased("object".to_string()));
    }
    binding
}

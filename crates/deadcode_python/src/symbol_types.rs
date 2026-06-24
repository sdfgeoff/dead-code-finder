use ruff_python_ast as ast;

use crate::symbol_index::{ImportTarget, ResolvedImport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TypeBinding {
    pub(super) base: String,
    pub(super) args: Vec<String>,
}

impl TypeBinding {
    pub(super) fn erased(base: String) -> Self {
        Self {
            base,
            args: Vec::new(),
        }
    }
}

pub(super) fn constructor_type_name(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<String> {
    let ast::Expr::Call(call) = expr else {
        return None;
    };
    type_name_from_expr(module, imports, &call.func)
}

pub(super) fn type_binding_from_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    match expr {
        ast::Expr::Subscript(subscript) => {
            let base = type_name_from_expr(module, imports, &subscript.value)?;
            Some(TypeBinding {
                base,
                args: type_args_from_expr(module, imports, &subscript.slice),
            })
        }
        _ => type_name_from_expr(module, imports, expr).map(TypeBinding::erased),
    }
}

pub(super) fn type_name_from_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<String> {
    match expr {
        ast::Expr::Name(name) => resolve_name_to_symbol(module, imports, name.id.as_str()),
        ast::Expr::Attribute(attribute) => dotted_expr(attribute).and_then(|dotted| {
            imports.iter().find_map(|import| {
                let ImportTarget::Module {
                    module,
                    external: false,
                } = &import.target
                else {
                    return None;
                };
                dotted
                    .strip_prefix(&import.binding)
                    .and_then(|suffix| suffix.strip_prefix('.'))
                    .map(|suffix| format!("{module}.{suffix}"))
            })
        }),
        ast::Expr::Subscript(subscript) => type_name_from_expr(module, imports, &subscript.value),
        _ => None,
    }
}

fn type_args_from_expr(module: &str, imports: &[ResolvedImport], expr: &ast::Expr) -> Vec<String> {
    match expr {
        ast::Expr::Tuple(tuple) => tuple
            .elts
            .iter()
            .filter_map(|expr| type_name_from_expr(module, imports, expr))
            .collect(),
        expr => type_name_from_expr(module, imports, expr)
            .into_iter()
            .collect(),
    }
}

fn resolve_name_to_symbol(module: &str, imports: &[ResolvedImport], name: &str) -> Option<String> {
    for import in imports.iter() {
        if import.binding != name {
            continue;
        }
        return match &import.target {
            ImportTarget::Symbol {
                module,
                name,
                external: false,
            } => Some(format!("{module}.{name}")),
            ImportTarget::Module {
                module,
                external: false,
            } => Some(module.clone()),
            _ => None,
        };
    }
    Some(format!("{module}.{name}"))
}

fn dotted_expr(expr: &ast::ExprAttribute) -> Option<String> {
    let mut parts = vec![expr.attr.as_str().to_string()];
    let mut value = expr.value.as_ref();
    loop {
        match value {
            ast::Expr::Name(name) => {
                parts.push(name.id.as_str().to_string());
                parts.reverse();
                return Some(parts.join("."));
            }
            ast::Expr::Attribute(attribute) => {
                parts.push(attribute.attr.as_str().to_string());
                value = attribute.value.as_ref();
            }
            _ => return None,
        }
    }
}

use ruff_python_ast as ast;

use crate::symbol_index::{ImportTarget, ResolvedImport, TypeBinding};

pub(super) fn type_binding_from_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    match expr {
        ast::Expr::BinOp(bin_op) if bin_op.op == ast::Operator::BitOr => Some(TypeBinding {
            base: "typing.Union".to_string(),
            args: [
                type_binding_from_expr(module, imports, &bin_op.left),
                type_binding_from_expr(module, imports, &bin_op.right),
            ]
            .into_iter()
            .flatten()
            .collect(),
            external: false,
        }),
        ast::Expr::Subscript(subscript) => {
            let base = type_binding_from_expr(module, imports, &subscript.value)?;
            let external = base.external && !is_typing_container(&base.base);
            Some(TypeBinding {
                external,
                base: base.base,
                args: type_args_from_expr(module, imports, &subscript.slice),
            })
        }
        _ => type_binding_from_name_expr(module, imports, expr),
    }
}

pub(super) fn type_binding_from_annotation_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    match expr {
        ast::Expr::StringLiteral(string) => {
            type_binding_from_annotation_string(module, imports, string.value.to_str())
        }
        ast::Expr::NoneLiteral(_) => Some(TypeBinding::erased("None".to_string())),
        _ => type_binding_from_expr(module, imports, expr),
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

fn type_binding_from_annotation_string(
    module: &str,
    imports: &[ResolvedImport],
    annotation: &str,
) -> Option<TypeBinding> {
    if annotation.is_empty() || annotation.contains('[') || annotation.contains('|') {
        return None;
    }
    if let Some((head, tail)) = annotation.split_once('.') {
        for import in imports {
            if import.binding != head {
                continue;
            }
            return match &import.target {
                ImportTarget::Module { module, external } => Some(TypeBinding {
                    base: format!("{module}.{tail}"),
                    args: Vec::new(),
                    external: *external,
                }),
                ImportTarget::Symbol { .. } | ImportTarget::Star { .. } => None,
            };
        }
    }
    resolve_name_to_type_binding(module, imports, annotation)
}

fn type_binding_from_name_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    match expr {
        ast::Expr::Name(name) => resolve_name_to_type_binding(module, imports, name.id.as_str()),
        ast::Expr::Attribute(attribute) => dotted_expr(attribute).and_then(|dotted| {
            imports.iter().find_map(|import| {
                let ImportTarget::Module { module, external } = &import.target else {
                    return None;
                };
                dotted
                    .strip_prefix(&import.binding)
                    .and_then(|suffix| suffix.strip_prefix('.'))
                    .map(|suffix| TypeBinding {
                        base: format!("{module}.{suffix}"),
                        args: Vec::new(),
                        external: *external,
                    })
            })
        }),
        ast::Expr::Subscript(subscript) => {
            type_binding_from_expr(module, imports, &subscript.value)
        }
        _ => None,
    }
}

fn type_args_from_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Vec<TypeBinding> {
    match expr {
        ast::Expr::Tuple(tuple) => tuple
            .elts
            .iter()
            .flat_map(|expr| callable_arg_list_or_type(module, imports, expr))
            .collect(),
        expr => type_binding_from_expr(module, imports, expr)
            .into_iter()
            .collect(),
    }
}

fn callable_arg_list_or_type(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Vec<TypeBinding> {
    match expr {
        ast::Expr::List(list) => list
            .elts
            .iter()
            .filter_map(|expr| type_binding_from_expr(module, imports, expr))
            .collect(),
        expr => type_binding_from_expr(module, imports, expr)
            .into_iter()
            .collect(),
    }
}

fn resolve_name_to_type_binding(
    module: &str,
    imports: &[ResolvedImport],
    name: &str,
) -> Option<TypeBinding> {
    if is_builtin_type(name) {
        return Some(TypeBinding::erased(name.to_string()));
    }
    for import in imports.iter() {
        if import.binding != name {
            continue;
        }
        return match &import.target {
            ImportTarget::Symbol {
                module,
                name,
                external,
            } => Some(TypeBinding {
                base: format!("{module}.{name}"),
                args: Vec::new(),
                external: *external,
            }),
            ImportTarget::Module { module, external } => Some(TypeBinding {
                base: module.clone(),
                args: Vec::new(),
                external: *external,
            }),
            ImportTarget::Star { .. } => None,
        };
    }
    Some(TypeBinding::erased(format!("{module}.{name}")))
}

fn resolve_name_to_symbol(module: &str, imports: &[ResolvedImport], name: &str) -> Option<String> {
    if is_builtin_type(name) {
        return Some(name.to_string());
    }
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

fn is_builtin_type(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "bytes"
            | "complex"
            | "dict"
            | "float"
            | "frozenset"
            | "int"
            | "list"
            | "object"
            | "set"
            | "str"
            | "type"
            | "tuple"
    )
}

fn is_typing_container(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Optional"
            | "typing.Union"
            | "typing.List"
            | "typing.Dict"
            | "typing.Mapping"
            | "typing.Sequence"
            | "typing.Set"
            | "typing.Tuple"
            | "typing.Type"
            | "typing.Annotated"
            | "typing_extensions.Annotated"
            | "typing_extensions.Type"
    ) || type_name.ends_with(".Annotated")
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

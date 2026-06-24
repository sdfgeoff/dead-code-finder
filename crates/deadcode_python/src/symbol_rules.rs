use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use crate::config::RuleConfig;
use crate::symbol_index::{ImportTarget, ResolvedImport, TypeBinding};

use super::symbol_types::{type_binding_from_expr, type_name_from_expr};

pub(super) fn decorator_registers_function(
    rules: &RuleConfig,
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> bool {
    let ast::Expr::Call(call) = expr else {
        return false;
    };
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return false;
    };
    let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
        return false;
    };
    let Some(receiver_type) = types.get(receiver.id.as_str()) else {
        return false;
    };
    rules.decorators.iter().any(|rule| {
        rule.effect == "registerDecoratedFunction"
            && rule.receiver_type == receiver_type.base
            && rule
                .methods
                .iter()
                .any(|method| method == attribute.attr.as_str())
    })
}

pub(super) fn constructor_binding(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    let ast::Expr::Call(call) = expr else {
        return None;
    };
    if matches!(call.func.as_ref(), ast::Expr::Name(name) if name.id.as_str() == "open") {
        return Some(TypeBinding {
            base: "builtins.open".to_string(),
            args: Vec::new(),
            external: true,
        });
    }
    if let Some(constructed_type) = constructed_type_from_callee(module, imports, rules, &call.func)
    {
        return Some(TypeBinding::erased(constructed_type));
    }
    type_binding_from_expr(module, imports, &call.func)
}

pub(super) fn constructed_type_from_callee(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    callee: &ast::Expr,
) -> Option<String> {
    let callable = callable_identity(module, imports, callee);
    callable
        .as_deref()
        .and_then(|callable| {
            rules
                .constructors
                .iter()
                .find(|rule| rule.match_ == callable)
                .map(|rule| rule.produces_type.clone())
        })
        .or_else(|| type_name_from_expr(module, imports, callee))
}

pub(super) fn callable_identity(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<String> {
    match expr {
        ast::Expr::Name(name) => resolve_name_identity(module, imports, name.id.as_str()),
        ast::Expr::Attribute(attribute) => resolve_attribute_identity(module, imports, attribute),
        ast::Expr::Subscript(subscript) => callable_identity(module, imports, &subscript.value),
        _ => None,
    }
}

pub(super) fn callable_argument_references(
    rules: &RuleConfig,
    call: &ast::ExprCall,
    callee: Option<&str>,
    types: &HashMap<String, TypeBinding>,
) -> Vec<(String, TextRange)> {
    let mut references = Vec::new();
    for rule in &rules.calls {
        if rule.effect != "useCallableArgument" || !call_rule_matches(rule, call, callee, types) {
            continue;
        }
        let Some(ast::Expr::Name(name)) = call.arguments.args.get(rule.argument) else {
            continue;
        };
        references.push((name.id.as_str().to_string(), name.range));
    }
    references
}

fn call_rule_matches(
    rule: &crate::config::CallRule,
    call: &ast::ExprCall,
    callee: Option<&str>,
    types: &HashMap<String, TypeBinding>,
) -> bool {
    if rule
        .function
        .as_deref()
        .is_some_and(|function| Some(function) == callee)
    {
        return true;
    }
    let (Some(receiver_type), Some(method)) = (&rule.receiver_type, &rule.method) else {
        return false;
    };
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return false;
    };
    if attribute.attr.as_str() != method {
        return false;
    }
    let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
        return false;
    };
    types
        .get(receiver.id.as_str())
        .is_some_and(|binding| binding.base == *receiver_type)
}

fn resolve_name_identity(module: &str, imports: &[ResolvedImport], name: &str) -> Option<String> {
    for import in imports.iter() {
        if import.binding != name {
            continue;
        }
        return match &import.target {
            ImportTarget::Symbol { module, name, .. } => Some(format!("{module}.{name}")),
            ImportTarget::Module { module, .. } => Some(module.clone()),
            ImportTarget::Star { .. } => None,
        };
    }
    Some(format!("{module}.{name}"))
}

fn resolve_attribute_identity(
    module: &str,
    imports: &[ResolvedImport],
    attribute: &ast::ExprAttribute,
) -> Option<String> {
    let mut parts = vec![attribute.attr.as_str().to_string()];
    let mut value = attribute.value.as_ref();
    loop {
        match value {
            ast::Expr::Name(name) => {
                let base = resolve_name_identity(module, imports, name.id.as_str())?;
                parts.reverse();
                return Some(format!("{}.{}", base, parts.join(".")));
            }
            ast::Expr::Attribute(nested) => {
                parts.push(nested.attr.as_str().to_string());
                value = nested.value.as_ref();
            }
            _ => return None,
        }
    }
}

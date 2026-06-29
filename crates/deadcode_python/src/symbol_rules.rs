use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange};

use crate::config::RuleConfig;
use crate::symbol_index::{ImportTarget, ResolvedImport, TypeBinding};

use super::symbol_types::{type_binding_from_expr, type_name_from_expr};

pub(super) fn decorator_registers_function(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> bool {
    rules.decorators.iter().any(|rule| {
        matches!(
            rule.effect.as_str(),
            "registerDecoratedFunction" | "registerBoundaryFunction"
        ) && decorator_matches(rule, expr, module, imports, types)
    })
}

pub(super) fn decorator_marks_boundary_function(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> bool {
    rules.decorators.iter().any(|rule| {
        rule.effect == "registerBoundaryFunction"
            && decorator_matches(rule, expr, module, imports, types)
    })
}

pub(super) fn decorator_callable_wrapper_type(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<String> {
    rules.decorators.iter().find_map(|rule| {
        (rule.effect == "wrapWithCallableType"
            && decorator_matches(rule, expr, module, imports, types))
        .then(|| rule.callable_type.clone())
        .flatten()
    })
}

fn decorator_matches(
    rule: &crate::config::DecoratorRule,
    expr: &ast::Expr,
    module: &str,
    imports: &[ResolvedImport],
    types: &HashMap<String, TypeBinding>,
) -> bool {
    let callee = match expr {
        ast::Expr::Call(call) => call.func.as_ref(),
        expr => expr,
    };
    if let Some(function) = &rule.function {
        return callable_identity(module, imports, callee).as_deref() == Some(function.as_str());
    }
    let ast::Expr::Attribute(attribute) = callee else {
        return false;
    };
    let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
        return false;
    };
    let Some(receiver_type) = types.get(receiver.id.as_str()) else {
        return false;
    };
    rule.receiver_type.as_ref() == Some(&receiver_type.base)
        && rule
            .methods
            .iter()
            .any(|method| method == attribute.attr.as_str())
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

pub(super) fn factory_return_binding(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    let ast::Expr::Call(call) = expr else {
        return None;
    };
    let callable = callable_identity(module, imports, &call.func)?;
    for rule in rules
        .factory_returns
        .iter()
        .filter(|rule| rule.function == callable)
    {
        let Some(output_expr) = factory_output_expr(call, rule) else {
            continue;
        };
        let Some(output_type) = type_binding_from_expr(module, imports, output_expr) else {
            continue;
        };
        let return_type = match rule.return_container.as_deref() {
            Some("list") => TypeBinding {
                base: "list".to_string(),
                args: vec![output_type],
                external: false,
            },
            _ => output_type,
        };
        return Some(TypeBinding {
            base: "typing.Callable".to_string(),
            args: vec![return_type],
            external: false,
        });
    }
    None
}

fn factory_output_expr<'a>(
    call: &'a ast::ExprCall,
    rule: &crate::config::FactoryReturnRule,
) -> Option<&'a ast::Expr> {
    if let Some(position) = rule.type_position {
        if let Some(arg) = call.arguments.args.get(position) {
            return Some(arg);
        }
    }
    let output_keyword = call.arguments.keywords.iter().find(|keyword| {
        keyword
            .arg
            .as_ref()
            .is_some_and(|name| name.as_str() == rule.type_keyword)
    })?;
    Some(&output_keyword.value)
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
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    call: &ast::ExprCall,
    callee: Option<&str>,
    types: &HashMap<String, TypeBinding>,
) -> Vec<(String, TextRange)> {
    let mut references = Vec::new();
    for rule in &rules.calls {
        if !call_rule_matches(rule, call, callee, types) {
            continue;
        }
        match rule.effect.as_str() {
            "useCallableArgument" => {
                let Some(ast::Expr::Name(name)) = call.arguments.args.get(rule.argument) else {
                    continue;
                };
                references.push((name.id.as_str().to_string(), name.range));
            }
            "useArgumentMember" => {
                let Some(argument) = call.arguments.args.get(rule.argument) else {
                    continue;
                };
                let Some(member) = &rule.member else {
                    continue;
                };
                let Some(argument_type) = callable_identity(module, imports, argument) else {
                    continue;
                };
                references.push((format!("{argument_type}.{member}"), argument.range()));
            }
            _ => {}
        }
    }
    references
}

pub(super) fn callable_dependency_argument(
    module: &str,
    imports: &[ResolvedImport],
    rules: &RuleConfig,
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<String> {
    let ast::Expr::Call(call) = expr else {
        return None;
    };
    let callee = callable_identity(module, imports, &call.func);
    for rule in &rules.calls {
        if rule.effect != "useCallableArgument"
            || !call_rule_matches(rule, call, callee.as_deref(), types)
        {
            continue;
        }
        let Some(argument) = call.arguments.args.get(rule.argument) else {
            continue;
        };
        return callable_identity(module, imports, argument);
    }
    None
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

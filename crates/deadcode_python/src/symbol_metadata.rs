use ruff_python_ast as ast;

use crate::symbol_index::{
    ClassFieldInfo, ClassInfo, FieldAnnotation, FunctionParameter, FunctionSignature,
    ResolvedImport, TypeBinding, ValueBinding,
};

use super::symbol_aliases::expand_alias_binding;
use super::symbol_expr::self_attribute_name;
use super::symbol_types::{type_binding_from_annotation_expr, type_binding_from_expr};

pub(super) fn class_info(
    module: &str,
    imports: &[ResolvedImport],
    class: String,
    class_def: &ast::StmtClassDef,
    values: &[ValueBinding],
) -> ClassInfo {
    let type_params = class_type_params(module, imports, class_def);
    let bases = class_def
        .arguments
        .as_ref()
        .map(|arguments| {
            arguments
                .args
                .iter()
                .filter_map(|base| type_binding_from_expr(module, imports, base))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let fields = class_fields(module, imports, class_def, &type_params, values);
    ClassInfo {
        class,
        bases,
        type_params,
        fields,
    }
}

pub(super) fn function_signature(
    module: &str,
    imports: &[ResolvedImport],
    function: &str,
    function_def: &ast::StmtFunctionDef,
) -> FunctionSignature {
    let parameters = function_def
        .parameters
        .iter()
        .map(|parameter| {
            let parameter = parameter.as_parameter();
            FunctionParameter {
                name: parameter.name.as_str().to_string(),
                annotation: parameter.annotation().and_then(|annotation| {
                    type_binding_from_annotation_expr(module, imports, annotation)
                }),
            }
        })
        .collect();
    FunctionSignature {
        function: function.to_string(),
        parameters,
        return_type: function_def
            .returns
            .as_ref()
            .and_then(|returns| type_binding_from_annotation_expr(module, imports, returns)),
        concrete_return_type: None,
        validated_return_types: Vec::new(),
    }
}

fn class_type_params(
    module: &str,
    imports: &[ResolvedImport],
    class_def: &ast::StmtClassDef,
) -> Vec<String> {
    let inline_params: Vec<String> = class_def
        .type_params
        .as_deref()
        .map(|type_params| {
            type_params
                .iter()
                .filter_map(|type_param| match type_param {
                    ast::TypeParam::TypeVar(type_var) => Some(type_var.name.as_str().to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    if !inline_params.is_empty() {
        return inline_params;
    }
    class_def
        .arguments
        .as_ref()
        .and_then(|arguments| {
            arguments.args.iter().find_map(|base| {
                let ast::Expr::Subscript(subscript) = base else {
                    return None;
                };
                let base = type_binding_from_expr(module, imports, &subscript.value)?;
                is_generic_base(&base.base)
                    .then(|| type_param_names_from_expr(module, imports, &subscript.slice))
            })
        })
        .unwrap_or_default()
}

fn type_param_names_from_expr(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Vec<String> {
    match expr {
        ast::Expr::Tuple(tuple) => tuple
            .elts
            .iter()
            .filter_map(|expr| type_param_name(module, imports, expr))
            .collect(),
        expr => type_param_name(module, imports, expr).into_iter().collect(),
    }
}

fn type_param_name(module: &str, imports: &[ResolvedImport], expr: &ast::Expr) -> Option<String> {
    type_binding_from_expr(module, imports, expr).map(|binding| binding.base)
}

fn is_generic_base(type_name: &str) -> bool {
    matches!(type_name, "typing.Generic" | "Generic") || type_name.ends_with(".Generic")
}

fn class_fields(
    module: &str,
    imports: &[ResolvedImport],
    class_def: &ast::StmtClassDef,
    type_params: &[String],
    values: &[ValueBinding],
) -> Vec<ClassFieldInfo> {
    let mut fields = class_def
        .body
        .iter()
        .filter_map(|statement| {
            let ast::Stmt::AnnAssign(assign) = statement else {
                return None;
            };
            let name = target_name(&assign.target)?;
            let annotation =
                field_annotation(module, imports, &assign.annotation, type_params, values)?;
            Some(ClassFieldInfo {
                name: name.to_string(),
                annotation,
            })
        })
        .collect::<Vec<_>>();
    for field in init_self_fields(module, imports, class_def) {
        if !fields.iter().any(|existing| existing.name == field.name) {
            fields.push(field);
        }
    }
    for field in property_fields(module, imports, class_def, type_params, values) {
        if !fields.iter().any(|existing| existing.name == field.name) {
            fields.push(field);
        }
    }
    fields
}

fn property_fields(
    module: &str,
    imports: &[ResolvedImport],
    class_def: &ast::StmtClassDef,
    type_params: &[String],
    values: &[ValueBinding],
) -> Vec<ClassFieldInfo> {
    class_def
        .body
        .iter()
        .filter_map(|statement| {
            let ast::Stmt::FunctionDef(function) = statement else {
                return None;
            };
            if !function.decorator_list.iter().any(
                |decorator| matches!(&decorator.expression, ast::Expr::Name(name) if name.id.as_str() == "property"),
            ) {
                return None;
            }
            let returns = function.returns.as_ref()?;
            let annotation = field_annotation(module, imports, returns, type_params, values)?;
            Some(ClassFieldInfo {
                name: function.name.as_str().to_string(),
                annotation,
            })
        })
        .collect()
}

fn init_self_fields(
    module: &str,
    imports: &[ResolvedImport],
    class_def: &ast::StmtClassDef,
) -> Vec<ClassFieldInfo> {
    class_def
        .body
        .iter()
        .find_map(|statement| {
            let ast::Stmt::FunctionDef(function) = statement else {
                return None;
            };
            (function.name.as_str() == "__init__").then_some(function)
        })
        .map(|function| {
            let parameter_types = function
                .parameters
                .iter()
                .filter_map(|parameter| {
                    let parameter = parameter.as_parameter();
                    let annotation = parameter.annotation()?;
                    let type_name = type_binding_from_annotation_expr(module, imports, annotation)?;
                    Some((parameter.name.as_str().to_string(), type_name))
                })
                .collect::<Vec<_>>();
            function
                .body
                .iter()
                .filter_map(|statement| {
                    init_self_field(module, imports, statement, &parameter_types)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn init_self_field(
    module: &str,
    imports: &[ResolvedImport],
    statement: &ast::Stmt,
    parameter_types: &[(String, TypeBinding)],
) -> Option<ClassFieldInfo> {
    let (field_name, type_name) = match statement {
        ast::Stmt::Assign(assign) => {
            if assign.targets.len() != 1 {
                return None;
            }
            let target = assign.targets.first()?;
            let field_name = self_attribute_name(target)?;
            let type_name = match assign.value.as_ref() {
                ast::Expr::Name(value) => parameter_types
                    .iter()
                    .find(|(parameter, _)| parameter == value.id.as_str())
                    .map(|(_, type_name)| type_name.clone())?,
                ast::Expr::Call(call) => {
                    callable_parameter_return_type(&call.func, parameter_types)
                        .or_else(|| type_binding_from_expr(module, imports, &call.func))?
                }
                value => coalesced_constructor_type(module, imports, value, parameter_types)
                    .or_else(|| collection_constructor_type(module, imports, value))?,
            };
            (field_name, type_name)
        }
        ast::Stmt::AnnAssign(assign) => {
            let field_name = self_attribute_name(&assign.target)?;
            let type_name = type_binding_from_annotation_expr(module, imports, &assign.annotation)?;
            (field_name, type_name)
        }
        _ => return None,
    };
    Some(ClassFieldInfo {
        name: field_name.to_string(),
        annotation: FieldAnnotation::Concrete(type_name),
    })
}

fn callable_parameter_return_type(
    func: &ast::Expr,
    parameter_types: &[(String, TypeBinding)],
) -> Option<TypeBinding> {
    let ast::Expr::Name(name) = func else {
        return None;
    };
    let binding = parameter_types
        .iter()
        .find(|(parameter, _)| parameter == name.id.as_str())
        .map(|(_, binding)| binding)?;
    is_callable_type(&binding.base)
        .then(|| binding.args.last().cloned())
        .flatten()
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    )
}

fn coalesced_constructor_type(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
    parameter_types: &[(String, TypeBinding)],
) -> Option<TypeBinding> {
    let ast::Expr::BoolOp(bool_op) = expr else {
        return None;
    };
    if bool_op.op != ast::BoolOp::Or || bool_op.values.len() != 2 {
        return None;
    }
    let left = parameter_binding(&bool_op.values[0], parameter_types);
    let right = constructed_value_type(module, imports, &bool_op.values[1]);
    match (left, right) {
        (Some(parameter), Some(constructed)) if union_contains(&parameter, &constructed.base) => {
            Some(constructed)
        }
        _ => None,
    }
}

fn parameter_binding(
    expr: &ast::Expr,
    parameter_types: &[(String, TypeBinding)],
) -> Option<TypeBinding> {
    let ast::Expr::Name(name) = expr else {
        return None;
    };
    parameter_types
        .iter()
        .find(|(parameter, _)| parameter == name.id.as_str())
        .map(|(_, binding)| binding.clone())
}

fn union_contains(binding: &TypeBinding, type_name: &str) -> bool {
    is_union_type(&binding.base) && binding.args.iter().any(|arg| arg.base == type_name)
}

fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "typing.Optional" | "Union" | "Optional"
    ) || type_name.ends_with(".Union")
        || type_name.ends_with(".Optional")
}

fn collection_constructor_type(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    match expr {
        ast::Expr::List(list) => list
            .elts
            .iter()
            .find_map(|element| constructed_value_type(module, imports, element))
            .map(|item| TypeBinding {
                base: "list".to_string(),
                args: vec![item],
                external: false,
            }),
        ast::Expr::ListComp(list_comp) => constructed_value_type(module, imports, &list_comp.elt)
            .map(|item| TypeBinding {
                base: "list".to_string(),
                args: vec![item],
                external: false,
            }),
        ast::Expr::Dict(dict) => dict
            .items
            .iter()
            .find_map(|item| constructed_value_type(module, imports, &item.value))
            .map(dict_with_value_type),
        ast::Expr::DictComp(dict_comp) => {
            constructed_value_type(module, imports, &dict_comp.value).map(dict_with_value_type)
        }
        _ => None,
    }
}

fn constructed_value_type(
    module: &str,
    imports: &[ResolvedImport],
    expr: &ast::Expr,
) -> Option<TypeBinding> {
    let ast::Expr::Call(call) = expr else {
        return None;
    };
    type_binding_from_expr(module, imports, &call.func)
}

fn dict_with_value_type(value: TypeBinding) -> TypeBinding {
    TypeBinding {
        base: "dict".to_string(),
        args: vec![TypeBinding::erased("object".to_string()), value],
        external: false,
    }
}

fn field_annotation(
    module: &str,
    imports: &[ResolvedImport],
    annotation: &ast::Expr,
    type_params: &[String],
    values: &[ValueBinding],
) -> Option<FieldAnnotation> {
    let mut binding = type_binding_from_annotation_expr(module, imports, annotation)?;
    rewrite_type_params(module, &mut binding, type_params);
    if !contains_type_param(&binding, type_params) {
        binding = expand_alias_binding(&binding, values);
        rewrite_type_params(module, &mut binding, type_params);
    }
    Some(FieldAnnotation::Concrete(binding))
}

fn contains_type_param(binding: &TypeBinding, type_params: &[String]) -> bool {
    type_params
        .iter()
        .any(|type_param| binding.base == *type_param)
        || binding
            .args
            .iter()
            .any(|arg| contains_type_param(arg, type_params))
}

fn rewrite_type_params(module: &str, binding: &mut TypeBinding, type_params: &[String]) {
    for type_param in type_params {
        if binding.base == *type_param {
            break;
        }
        if binding.base == format!("{module}.{type_param}") {
            binding.base = type_param.clone();
            break;
        }
    }
    for arg in &mut binding.args {
        rewrite_type_params(module, arg, type_params);
    }
}

fn target_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        _ => None,
    }
}

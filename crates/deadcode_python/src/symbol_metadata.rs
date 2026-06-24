use ruff_python_ast as ast;

use crate::symbol_index::{
    ClassFieldInfo, ClassInfo, FieldAnnotation, FunctionSignature, ResolvedImport, TypeBinding,
};

use super::symbol_expr::self_attribute_name;
use super::symbol_types::{type_binding_from_expr, type_name_from_expr};

pub(super) fn class_info(
    module: &str,
    imports: &[ResolvedImport],
    class: String,
    class_def: &ast::StmtClassDef,
) -> ClassInfo {
    let type_params = class_type_params(class_def);
    let bases = class_def
        .arguments
        .as_ref()
        .map(|arguments| {
            arguments
                .args
                .iter()
                .filter_map(|base| type_name_from_expr(module, imports, base))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let fields = class_fields(module, imports, class_def, &type_params);
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
    let parameter_types = function_def
        .parameters
        .iter()
        .map(|parameter| {
            parameter
                .as_parameter()
                .annotation()
                .and_then(|annotation| type_name_from_expr(module, imports, annotation))
        })
        .collect();
    FunctionSignature {
        function: function.to_string(),
        parameter_types,
        return_type: function_def
            .returns
            .as_ref()
            .and_then(|returns| type_binding_from_expr(module, imports, returns)),
    }
}

fn class_type_params(class_def: &ast::StmtClassDef) -> Vec<String> {
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
                let ast::Expr::Name(name) = subscript.value.as_ref() else {
                    return None;
                };
                (name.id.as_str() == "Generic")
                    .then(|| type_param_names_from_expr(&subscript.slice))
            })
        })
        .unwrap_or_default()
}

fn type_param_names_from_expr(expr: &ast::Expr) -> Vec<String> {
    match expr {
        ast::Expr::Tuple(tuple) => tuple.elts.iter().filter_map(type_param_name).collect(),
        expr => type_param_name(expr).into_iter().collect(),
    }
}

fn type_param_name(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str().to_string()),
        _ => None,
    }
}

fn class_fields(
    module: &str,
    imports: &[ResolvedImport],
    class_def: &ast::StmtClassDef,
    type_params: &[String],
) -> Vec<ClassFieldInfo> {
    let mut fields = class_def
        .body
        .iter()
        .filter_map(|statement| {
            let ast::Stmt::AnnAssign(assign) = statement else {
                return None;
            };
            let name = target_name(&assign.target)?;
            let annotation = field_annotation(module, imports, &assign.annotation, type_params)?;
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
    fields
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
                    let type_name = type_binding_from_expr(module, imports, annotation)?;
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
    let ast::Stmt::Assign(assign) = statement else {
        return None;
    };
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
        ast::Expr::Call(call) => type_binding_from_expr(module, imports, &call.func)?,
        _ => return None,
    };
    Some(ClassFieldInfo {
        name: field_name.to_string(),
        annotation: FieldAnnotation::Concrete(type_name),
    })
}

fn field_annotation(
    module: &str,
    imports: &[ResolvedImport],
    annotation: &ast::Expr,
    type_params: &[String],
) -> Option<FieldAnnotation> {
    let mut binding = type_binding_from_expr(module, imports, annotation)?;
    rewrite_type_params(module, &mut binding, type_params);
    Some(FieldAnnotation::Concrete(binding))
}

fn rewrite_type_params(module: &str, binding: &mut TypeBinding, type_params: &[String]) {
    for type_param in type_params {
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

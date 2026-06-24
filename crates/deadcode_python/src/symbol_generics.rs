use std::collections::HashMap;

use ruff_python_ast as ast;

use crate::symbol_index::{ClassInfo, FieldAnnotation, TypeBinding};

pub(super) fn field_read_type(
    classes: &[ClassInfo],
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Attribute(attribute) = expr else {
        return None;
    };
    let receiver_type = match attribute.value.as_ref() {
        ast::Expr::Name(receiver) => types.get(receiver.id.as_str()).cloned(),
        expr => expr_type(classes, expr, types),
    }?;
    let class_info = classes
        .iter()
        .find(|class_info| class_info.class == receiver_type.base)?;
    let field = class_info
        .fields
        .iter()
        .find(|field| field.name == attribute.attr.as_str())?;
    match &field.annotation {
        FieldAnnotation::Concrete(binding) => {
            Some(substitute_type_params(binding, class_info, &receiver_type))
        }
    }
}

pub(super) fn expr_type(
    classes: &[ClassInfo],
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    match expr {
        ast::Expr::Name(name) => types.get(name.id.as_str()).cloned(),
        ast::Expr::Attribute(_) => field_read_type(classes, expr, types),
        ast::Expr::Subscript(subscript) => {
            collection_item_type(&expr_type(classes, &subscript.value, types)?)
        }
        ast::Expr::Await(await_expr) => expr_type(classes, &await_expr.value, types),
        ast::Expr::Call(call) => mapping_get_call_type(classes, call, types),
        ast::Expr::List(list) => {
            list_item_type(classes, &list.elts, types).map(|item| TypeBinding {
                base: "list".to_string(),
                args: vec![item],
                external: false,
            })
        }
        _ => None,
    }
}

pub(super) fn iterable_item_type(
    classes: &[ClassInfo],
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    collection_item_type(&expr_type(classes, expr, types)?)
}

fn substitute_type_params(
    binding: &TypeBinding,
    class_info: &ClassInfo,
    receiver_type: &TypeBinding,
) -> TypeBinding {
    if let Some(position) = class_info
        .type_params
        .iter()
        .position(|candidate| candidate == &binding.base)
    {
        return receiver_type
            .args
            .get(position)
            .cloned()
            .unwrap_or_else(|| TypeBinding::erased(binding.base.clone()));
    }
    TypeBinding {
        base: binding.base.clone(),
        args: binding
            .args
            .iter()
            .map(|arg| substitute_type_params(arg, class_info, receiver_type))
            .collect(),
        external: binding.external,
    }
}

fn is_iterable_collection(type_name: &str) -> bool {
    matches!(
        type_name,
        "list"
            | "set"
            | "tuple"
            | "typing.List"
            | "typing.Sequence"
            | "typing.Set"
            | "typing.Tuple"
    ) || type_name.ends_with(".list")
        || type_name.ends_with(".set")
        || type_name.ends_with(".tuple")
}

fn collection_item_type(collection_type: &TypeBinding) -> Option<TypeBinding> {
    if is_mapping_collection(&collection_type.base) {
        return collection_type.args.get(1).cloned();
    }
    if is_iterable_collection(&collection_type.base) {
        return collection_type.args.first().cloned();
    }
    None
}

fn mapping_get_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    if attribute.attr.as_str() != "get" {
        return None;
    }
    let receiver_type = expr_type(classes, &attribute.value, types)?;
    if is_mapping_collection(&receiver_type.base) {
        return receiver_type.args.get(1).cloned();
    }
    None
}

fn is_mapping_collection(type_name: &str) -> bool {
    matches!(type_name, "dict" | "typing.Dict" | "typing.Mapping") || type_name.ends_with(".dict")
}

fn list_item_type(
    classes: &[ClassInfo],
    elements: &[ast::Expr],
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let mut item_type = None;
    for element in elements {
        let element_type = expr_type(classes, element, types)?;
        if item_type
            .as_ref()
            .is_some_and(|existing: &TypeBinding| existing != &element_type)
        {
            return None;
        }
        item_type = Some(element_type);
    }
    item_type
}

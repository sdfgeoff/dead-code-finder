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
    let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
        return None;
    };
    let receiver_type = types.get(receiver.id.as_str())?;
    let class_info = classes
        .iter()
        .find(|class_info| class_info.class == receiver_type.base)?;
    let field = class_info
        .fields
        .iter()
        .find(|field| field.name == attribute.attr.as_str())?;
    match &field.annotation {
        FieldAnnotation::Concrete(binding) => {
            Some(substitute_type_params(binding, class_info, receiver_type))
        }
    }
}

pub(super) fn iterable_item_type(
    classes: &[ClassInfo],
    expr: &ast::Expr,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let iterable_type = field_read_type(classes, expr, types).or_else(|| match expr {
        ast::Expr::Name(name) => types.get(name.id.as_str()).cloned(),
        _ => None,
    })?;
    if is_iterable_collection(&iterable_type.base) {
        return iterable_type.args.first().cloned();
    }
    None
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

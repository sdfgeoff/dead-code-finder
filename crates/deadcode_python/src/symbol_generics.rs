use std::collections::HashMap;

use ruff_python_ast as ast;

use crate::symbol_index::{ClassInfo, FieldAnnotation};

use super::symbol_types::TypeBinding;

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
        FieldAnnotation::Concrete(type_name) => Some(TypeBinding::erased(type_name.clone())),
        FieldAnnotation::TypeParam(type_param) => class_info
            .type_params
            .iter()
            .position(|candidate| candidate == type_param)
            .and_then(|position| receiver_type.args.get(position))
            .cloned()
            .map(TypeBinding::erased),
    }
}

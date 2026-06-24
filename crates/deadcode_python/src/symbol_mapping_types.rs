use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_generics::expr_type;
use crate::symbol_index::{ClassInfo, TypeBinding};

pub(super) fn mapping_items_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    if attribute.attr.as_str() != "items" {
        return None;
    }
    let receiver_type = expr_type(classes, &attribute.value, types)?;
    if !is_mapping_collection(&receiver_type.base) {
        return None;
    }
    Some(TypeBinding {
        base: "list".to_string(),
        args: vec![TypeBinding {
            base: "tuple".to_string(),
            args: receiver_type.args,
            external: false,
        }],
        external: false,
    })
}

pub(super) fn mapping_value_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    if !matches!(attribute.attr.as_str(), "get" | "setdefault") {
        return None;
    }
    let receiver_type = expr_type(classes, &attribute.value, types)?;
    if is_mapping_collection(&receiver_type.base) {
        return receiver_type.args.get(1).cloned();
    }
    None
}

pub(super) fn mapping_values_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    if attribute.attr.as_str() != "values" {
        return None;
    }
    let receiver_type = expr_type(classes, &attribute.value, types)?;
    if !is_mapping_collection(&receiver_type.base) {
        return None;
    }
    Some(TypeBinding {
        base: "list".to_string(),
        args: receiver_type.args.get(1).cloned().into_iter().collect(),
        external: false,
    })
}

pub(super) fn is_mapping_collection(type_name: &str) -> bool {
    matches!(type_name, "dict" | "typing.Dict" | "typing.Mapping") || type_name.ends_with(".dict")
}

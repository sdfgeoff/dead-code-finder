use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_comprehension_narrowing::apply_isinstance_narrowing;
use super::symbol_expr::target_name;
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
    if receiver_type.external {
        return Some(TypeBinding {
            base: format!("{}.{}", receiver_type.base, attribute.attr.as_str()),
            args: Vec::new(),
            external: true,
        });
    }
    let Some(class_info) = classes
        .iter()
        .find(|class_info| class_info.class == receiver_type.base)
    else {
        return union_field_read_type(classes, &receiver_type, attribute.attr.as_str());
    };
    class_or_base_field_type(classes, class_info, &receiver_type, attribute.attr.as_str())
}

fn union_field_read_type(
    classes: &[ClassInfo],
    receiver_type: &TypeBinding,
    field_name: &str,
) -> Option<TypeBinding> {
    if !is_union_type(&receiver_type.base) {
        return None;
    }
    receiver_type
        .args
        .iter()
        .filter(|arg| !is_none_type(&arg.base))
        .find_map(|arg| field_type_for_class(classes, arg, field_name))
}

fn field_type_for_class(
    classes: &[ClassInfo],
    receiver_type: &TypeBinding,
    field_name: &str,
) -> Option<TypeBinding> {
    let class_info = classes
        .iter()
        .find(|class_info| class_info.class == receiver_type.base)?;
    class_or_base_field_type(classes, class_info, receiver_type, field_name)
}

pub(super) fn field_type_for_receiver(
    classes: &[ClassInfo],
    receiver_type: &TypeBinding,
    field_name: &str,
) -> Option<TypeBinding> {
    if receiver_type.external {
        return Some(TypeBinding {
            base: format!("{}.{}", receiver_type.base, field_name),
            args: Vec::new(),
            external: true,
        });
    }
    let class_info = classes
        .iter()
        .find(|class_info| class_info.class == receiver_type.base)?;
    class_or_base_field_type(classes, class_info, receiver_type, field_name)
}

fn class_or_base_field_type(
    classes: &[ClassInfo],
    class_info: &ClassInfo,
    receiver_type: &TypeBinding,
    field_name: &str,
) -> Option<TypeBinding> {
    class_field_type(class_info, receiver_type, field_name).or_else(|| {
        class_info.bases.iter().find_map(|base| {
            let base_info = classes
                .iter()
                .find(|class_info| class_info.class == base.base)?;
            class_or_base_field_type(classes, base_info, base, field_name)
        })
    })
}

fn class_field_type(
    class_info: &ClassInfo,
    receiver_type: &TypeBinding,
    field_name: &str,
) -> Option<TypeBinding> {
    let field = class_info
        .fields
        .iter()
        .find(|field| field.name == field_name)?;
    match &field.annotation {
        FieldAnnotation::Concrete(binding) => {
            Some(substitute_type_params(binding, class_info, receiver_type))
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
        ast::Expr::Call(call) => builtin_constructor_call_type(classes, call, types)
            .or_else(|| zip_call_type(classes, call, types))
            .or_else(|| enumerate_call_type(classes, call, types))
            .or_else(|| mapping_items_call_type(classes, call, types))
            .or_else(|| mapping_values_call_type(classes, call, types))
            .or_else(|| mapping_value_call_type(classes, call, types))
            .or_else(|| builtin_method_call_type(classes, call, types))
            .or_else(|| callable_call_return_type(classes, call, types))
            .or_else(|| unique_class_constructor_type(classes, call)),
        ast::Expr::List(list) => {
            list_item_type(classes, &list.elts, types).map(|item| TypeBinding {
                base: "list".to_string(),
                args: vec![item],
                external: false,
            })
        }
        ast::Expr::ListComp(list_comp) => list_comprehension_type(classes, list_comp, types),
        ast::Expr::Dict(dict) => dict_type(classes, &dict.items, types),
        ast::Expr::DictComp(dict_comp) => dict_comprehension_type(classes, dict_comp, types),
        ast::Expr::NoneLiteral(_) => Some(TypeBinding::erased("None".to_string())),
        ast::Expr::StringLiteral(_) | ast::Expr::FString(_) => Some(TypeBinding {
            base: "str".to_string(),
            args: Vec::new(),
            external: false,
        }),
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

fn zip_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Name(name) = call.func.as_ref() else {
        return None;
    };
    if name.id.as_str() != "zip" || call.arguments.args.is_empty() {
        return None;
    }
    let tuple_args = call
        .arguments
        .args
        .iter()
        .map(|arg| {
            iterable_item_type(classes, arg, types)
                .unwrap_or_else(|| TypeBinding::erased("object".to_string()))
        })
        .collect();
    Some(TypeBinding {
        base: "list".to_string(),
        args: vec![TypeBinding {
            base: "tuple".to_string(),
            args: tuple_args,
            external: false,
        }],
        external: false,
    })
}

fn enumerate_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Name(name) = call.func.as_ref() else {
        return None;
    };
    if name.id.as_str() != "enumerate" {
        return None;
    }
    let item_type = call
        .arguments
        .args
        .first()
        .and_then(|arg| iterable_item_type(classes, arg, types))?;
    Some(TypeBinding {
        base: "list".to_string(),
        args: vec![TypeBinding {
            base: "tuple".to_string(),
            args: vec![TypeBinding::erased("int".to_string()), item_type],
            external: false,
        }],
        external: false,
    })
}

pub(super) fn member_reference_target_bases(receiver_type: &TypeBinding) -> Vec<String> {
    if !is_union_type(&receiver_type.base) {
        return vec![receiver_type.base.clone()];
    }
    receiver_type
        .args
        .iter()
        .filter(|arg| !arg.external && !is_none_type(&arg.base))
        .map(|arg| arg.base.clone())
        .collect()
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
            | "collections.abc.Sequence"
            | "collections.abc.Iterable"
            | "collections.abc.Collection"
            | "collections.abc.AsyncIterator"
            | "collections.abc.AsyncIterable"
            | "typing.AsyncIterator"
            | "typing.AsyncIterable"
    ) || type_name.ends_with(".list")
        || type_name.ends_with(".set")
        || type_name.ends_with(".tuple")
}

fn is_union_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Union" | "types.UnionType" | "typing.Optional" | "Optional"
    )
}

fn is_none_type(type_name: &str) -> bool {
    matches!(type_name, "None" | "builtins.None")
        || type_name.ends_with(".None")
        || type_name.ends_with(".NoneType")
}

pub(super) fn collection_item_type(collection_type: &TypeBinding) -> Option<TypeBinding> {
    if let Some(inner) = non_none_union_member(collection_type) {
        return collection_item_type(inner);
    }
    if is_mapping_collection(&collection_type.base) {
        return collection_type.args.get(1).cloned();
    }
    if is_iterable_collection(&collection_type.base) {
        return collection_type.args.first().cloned();
    }
    if collection_type.external {
        return Some(TypeBinding {
            base: format!("{}.__item__", collection_type.base),
            args: Vec::new(),
            external: true,
        });
    }
    None
}

fn builtin_constructor_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Name(name) = call.func.as_ref() else {
        return None;
    };
    if matches!(name.id.as_str(), "list" | "set" | "tuple") {
        let item_type = call
            .arguments
            .args
            .first()
            .and_then(|arg| iterable_item_type(classes, arg, types));
        return Some(TypeBinding {
            base: name.id.as_str().to_string(),
            args: item_type.into_iter().collect(),
            external: false,
        });
    }
    matches!(
        name.id.as_str(),
        "bool" | "bytes" | "complex" | "float" | "int" | "str"
    )
    .then(|| TypeBinding::erased(name.id.as_str().to_string()))
}

fn mapping_items_call_type(
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

fn mapping_value_call_type(
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

fn mapping_values_call_type(
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

fn builtin_method_call_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    let receiver_type = expr_type(classes, &attribute.value, types)?;
    let receiver_type = non_none_union_member(&receiver_type).unwrap_or(&receiver_type);
    if attribute.attr.as_str() == "model_dump_json" {
        return Some(TypeBinding::erased("str".to_string()));
    }
    match (receiver_type.base.as_str(), attribute.attr.as_str()) {
        ("str", "split" | "rsplit") => Some(TypeBinding {
            base: "list".to_string(),
            args: vec![TypeBinding::erased("str".to_string())],
            external: false,
        }),
        ("str", "join" | "strip" | "replace") | ("bytes", "decode") => {
            Some(TypeBinding::erased("str".to_string()))
        }
        ("str", "encode") => Some(TypeBinding::erased("bytes".to_string())),
        ("str", "startswith") => Some(TypeBinding::erased("bool".to_string())),
        _ => None,
    }
}

fn non_none_union_member(binding: &TypeBinding) -> Option<&TypeBinding> {
    if !is_union_type(&binding.base) {
        return None;
    }
    let mut non_none = binding.args.iter().filter(|arg| !is_none_type(&arg.base));
    let member = non_none.next()?;
    non_none.next().is_none().then_some(member)
}

fn callable_call_return_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let callable_type = expr_type(classes, &call.func, types)?;
    if !is_callable_type(&callable_type.base) {
        return None;
    }
    callable_type.args.last().cloned()
}

fn unique_class_constructor_type(
    classes: &[ClassInfo],
    call: &ast::ExprCall,
) -> Option<TypeBinding> {
    let ast::Expr::Name(name) = call.func.as_ref() else {
        return None;
    };
    let suffix = format!(".{}", name.id.as_str());
    let mut matches = classes
        .iter()
        .filter(|class_info| class_info.class.ends_with(&suffix));
    let class_info = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    Some(TypeBinding::erased(class_info.class.clone()))
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    )
}

fn is_mapping_collection(type_name: &str) -> bool {
    matches!(type_name, "dict" | "typing.Dict" | "typing.Mapping") || type_name.ends_with(".dict")
}

fn dict_type(
    classes: &[ClassInfo],
    items: &[ast::DictItem],
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let mut key_type = None;
    let mut value_type = None;
    for item in items {
        let key = item
            .key
            .as_ref()
            .and_then(|key| expr_type(classes, key, types))
            .unwrap_or_else(|| TypeBinding::erased("object".to_string()));
        let value = expr_type(classes, &item.value, types)?;
        if key_type
            .as_ref()
            .is_some_and(|existing: &TypeBinding| existing != &key)
            || value_type
                .as_ref()
                .is_some_and(|existing: &TypeBinding| existing != &value)
        {
            return None;
        }
        key_type = Some(key);
        value_type = Some(value);
    }
    Some(TypeBinding {
        base: "dict".to_string(),
        args: vec![key_type?, value_type?],
        external: false,
    })
}

fn dict_comprehension_type(
    classes: &[ClassInfo],
    dict_comp: &ast::ExprDictComp,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let [generator] = dict_comp.generators.as_slice() else {
        return None;
    };
    let ast::Expr::Name(target) = &generator.target else {
        return None;
    };
    let item_type = iterable_item_type(classes, &generator.iter, types)?;
    let mut scoped_types = types.clone();
    scoped_types.insert(target.id.as_str().to_string(), item_type);
    let key_type = expr_type(classes, &dict_comp.key, &scoped_types)
        .unwrap_or_else(|| TypeBinding::erased("object".to_string()));
    Some(TypeBinding {
        base: "dict".to_string(),
        args: vec![
            key_type,
            expr_type(classes, &dict_comp.value, &scoped_types)?,
        ],
        external: false,
    })
}

fn list_comprehension_type(
    classes: &[ClassInfo],
    list_comp: &ast::ExprListComp,
    types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    let scoped_types = comprehension_types(classes, &list_comp.generators, types)?;
    Some(TypeBinding {
        base: "list".to_string(),
        args: vec![expr_type(classes, &list_comp.elt, &scoped_types)?],
        external: false,
    })
}

fn comprehension_types(
    classes: &[ClassInfo],
    generators: &[ast::Comprehension],
    types: &HashMap<String, TypeBinding>,
) -> Option<HashMap<String, TypeBinding>> {
    let mut scoped_types = types.clone();
    for generator in generators {
        let item_type = iterable_item_type(classes, &generator.iter, &scoped_types)?;
        bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
        for guard in &generator.ifs {
            apply_isinstance_narrowing(guard, &mut scoped_types);
        }
    }
    Some(scoped_types)
}

fn bind_comprehension_target(
    target: &ast::Expr,
    item_type: &TypeBinding,
    types: &mut HashMap<String, TypeBinding>,
) {
    if let Some(name) = target_name(target) {
        types.insert(name.to_string(), item_type.clone());
        return;
    }
    let tuple_items = match target {
        ast::Expr::Tuple(tuple) => &tuple.elts,
        ast::Expr::List(list) => &list.elts,
        _ => return,
    };
    if item_type.base != "tuple" || item_type.args.len() != tuple_items.len() {
        return;
    }
    for (target_item, binding) in tuple_items.iter().zip(&item_type.args) {
        if let Some(name) = target_name(target_item) {
            types.insert(name.to_string(), binding.clone());
        }
    }
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

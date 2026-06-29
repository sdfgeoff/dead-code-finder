use std::collections::{HashMap, HashSet};

use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange};

use super::super::AccessKind;
use super::symbol_generics::expr_type;
use super::symbol_members::push_member_reference;
use super::SymbolCollector;
use crate::symbol_index::{ClassInfo, FieldAnnotation, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn expand_model_dump_keyword(
        &mut self,
        owner: &str,
        constructor_type: &str,
        value: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
        range: TextRange,
    ) -> bool {
        let ast::Expr::Call(call) = value else {
            return false;
        };
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return false;
        };
        if attribute.attr.as_str() != "model_dump" {
            return false;
        }
        let excluded = model_dump_excludes(call);
        let source_type = expr_type(self.available_classes, &attribute.value, types)
            .filter(|binding| !binding.external);
        let source_fields = source_type
            .as_ref()
            .map(|binding| class_field_names(self.available_classes, &binding.base))
            .unwrap_or_default();
        let target_fields = class_field_names(self.available_classes, constructor_type);
        if target_fields.is_empty() {
            return false;
        }
        let expanded_fields = if source_fields.is_empty() {
            target_fields.iter().collect::<Vec<_>>()
        } else {
            source_fields
                .intersection(&target_fields)
                .collect::<Vec<_>>()
        };
        for field in expanded_fields
            .into_iter()
            .filter(|field| !excluded.contains(*field))
        {
            if source_fields.contains(field) {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    owner,
                    format!("{}.{}", source_type.as_ref().unwrap().base, field),
                    AccessKind::Read,
                    range,
                );
            }
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", constructor_type, field),
                AccessKind::Construct,
                range,
            );
        }
        true
    }

    pub(super) fn collect_model_dump_references(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) {
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return;
        };
        if !matches!(attribute.attr.as_str(), "model_dump" | "model_dump_json") {
            return;
        }
        let Some(binding) = expr_type(self.available_classes, &attribute.value, types)
            .filter(|binding| !binding.external)
        else {
            return;
        };
        self.collect_model_dump_binding(owner, &binding, call.range(), &mut Vec::new());
    }

    fn collect_model_dump_binding(
        &mut self,
        owner: &str,
        binding: &TypeBinding,
        range: TextRange,
        visited: &mut Vec<String>,
    ) {
        if is_transparent_container(&binding.base) {
            for arg in &binding.args {
                self.collect_model_dump_binding(owner, arg, range, visited);
            }
            return;
        }
        if binding.external
            || !class_derives_from(self.available_classes, &binding.base, "pydantic.BaseModel")
            || visited.iter().any(|visited| visited == &binding.base)
        {
            return;
        }
        visited.push(binding.base.clone());
        let fields = class_fields(self.available_classes, &binding.base);
        for (field_name, field_type) in fields {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", binding.base, field_name),
                AccessKind::Read,
                range,
            );
            self.collect_model_dump_binding(owner, &field_type, range, visited);
        }
        visited.pop();
    }
}

fn class_fields(classes: &[ClassInfo], class_name: &str) -> Vec<(String, TypeBinding)> {
    let mut fields = Vec::new();
    collect_class_fields(classes, class_name, &mut fields, &mut Vec::new());
    fields
}

fn collect_class_fields(
    classes: &[ClassInfo],
    class_name: &str,
    fields: &mut Vec<(String, TypeBinding)>,
    visited: &mut Vec<String>,
) {
    if visited.iter().any(|visited| visited == class_name) {
        return;
    }
    visited.push(class_name.to_string());
    let Some(class_info) = classes
        .iter()
        .find(|class_info| class_info.class == class_name)
    else {
        return;
    };
    for base in &class_info.bases {
        collect_class_fields(classes, &base.base, fields, visited);
    }
    fields.extend(
        class_info
            .fields
            .iter()
            .map(|field| match &field.annotation {
                FieldAnnotation::Concrete(binding) => (field.name.clone(), binding.clone()),
            }),
    );
}

fn is_transparent_container(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Annotated"
            | "typing_extensions.Annotated"
            | "typing.Union"
            | "types.UnionType"
            | "typing.Optional"
            | "Optional"
            | "list"
            | "typing.List"
            | "List"
    )
}

fn class_derives_from(classes: &[ClassInfo], concrete_type: &str, base_type: &str) -> bool {
    class_derives_from_inner(classes, concrete_type, base_type, &mut Vec::new())
}

fn class_derives_from_inner(
    classes: &[ClassInfo],
    concrete_type: &str,
    base_type: &str,
    visited: &mut Vec<String>,
) -> bool {
    if visited.iter().any(|visited| visited == concrete_type) {
        return false;
    }
    visited.push(concrete_type.to_string());
    let Some(class_info) = classes
        .iter()
        .find(|class_info| class_info.class == concrete_type)
    else {
        return false;
    };
    class_info.bases.iter().any(|base| {
        base.base == base_type || class_derives_from_inner(classes, &base.base, base_type, visited)
    })
}

fn class_field_names(classes: &[ClassInfo], class_name: &str) -> HashSet<String> {
    let mut fields = HashSet::new();
    collect_class_field_names(classes, class_name, &mut fields, &mut Vec::new());
    fields
}

fn collect_class_field_names(
    classes: &[ClassInfo],
    class_name: &str,
    fields: &mut HashSet<String>,
    visited: &mut Vec<String>,
) {
    if visited.iter().any(|visited| visited == class_name) {
        return;
    }
    visited.push(class_name.to_string());
    let Some(class_info) = classes
        .iter()
        .find(|class_info| class_info.class == class_name)
    else {
        return;
    };
    for field in &class_info.fields {
        fields.insert(field.name.clone());
    }
    for base in &class_info.bases {
        collect_class_field_names(classes, &base.base, fields, visited);
    }
}

fn model_dump_excludes(call: &ast::ExprCall) -> HashSet<String> {
    call.arguments
        .keywords
        .iter()
        .find(|keyword| {
            keyword
                .arg
                .as_ref()
                .is_some_and(|arg| arg.as_str() == "exclude")
        })
        .map(|keyword| string_collection(&keyword.value))
        .unwrap_or_default()
}

fn string_collection(expr: &ast::Expr) -> HashSet<String> {
    match expr {
        ast::Expr::Set(set) => set.elts.iter().filter_map(string_literal).collect(),
        ast::Expr::List(list) => list.elts.iter().filter_map(string_literal).collect(),
        ast::Expr::Tuple(tuple) => tuple.elts.iter().filter_map(string_literal).collect(),
        _ => HashSet::new(),
    }
}

fn string_literal(expr: &ast::Expr) -> Option<String> {
    let ast::Expr::StringLiteral(string) = expr else {
        return None;
    };
    Some(string.value.to_str().to_string())
}

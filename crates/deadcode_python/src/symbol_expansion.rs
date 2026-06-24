use std::collections::{HashMap, HashSet};

use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use super::super::AccessKind;
use super::symbol_generics::expr_type;
use super::symbol_members::push_member_reference;
use super::SymbolCollector;
use crate::symbol_index::{ClassInfo, TypeBinding};

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

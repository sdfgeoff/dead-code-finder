use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_members::push_member_reference;
use super::symbol_rules::callable_identity;
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, ClassInfo, FieldAnnotation, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn collect_factory_model_surfaces(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) {
        let Some(callable) = callable_identity(self.module, self.imports, &call.func) else {
            return;
        };
        let rules = self
            .rules
            .factory_returns
            .iter()
            .filter(|rule| rule.function == callable)
            .cloned()
            .collect::<Vec<_>>();
        for rule in rules {
            if rule.mark_input_fields {
                self.collect_factory_model_surface(
                    owner,
                    call,
                    rule.input_type_keyword.as_deref().unwrap_or("input"),
                    rule.input_type_position,
                    AccessKind::Read,
                    types,
                );
            }
            if rule.mark_output_fields {
                self.collect_factory_model_surface(
                    owner,
                    call,
                    &rule.type_keyword,
                    rule.type_position,
                    AccessKind::Construct,
                    types,
                );
            }
        }
    }

    fn collect_factory_model_surface(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        keyword_name: &str,
        position: Option<usize>,
        access: AccessKind,
        _types: &HashMap<String, TypeBinding>,
    ) {
        let Some((value, range)) = factory_model_argument(call, keyword_name, position) else {
            return;
        };
        let Some(binding) = type_binding_from_expr(self.module, self.imports, value) else {
            return;
        };
        self.collect_model_surface_binding(owner, &binding, access, range);
    }

    fn collect_model_surface_binding(
        &mut self,
        owner: &str,
        binding: &TypeBinding,
        access: AccessKind,
        range: ruff_text_size::TextRange,
    ) {
        let binding = expand_alias_binding(binding, self.available_values);
        if is_transparent_container(&binding.base) {
            for arg in &binding.args {
                self.collect_model_surface_binding(owner, arg, access.clone(), range);
            }
            return;
        }
        if binding.external {
            return;
        }
        let fields = class_fields(self.available_classes, &binding.base);
        for (field_name, field_type) in fields {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", binding.base, field_name),
                access.clone(),
                range,
            );
            self.collect_model_surface_binding(owner, &field_type, access.clone(), range);
        }
    }
}

fn factory_model_argument<'a>(
    call: &'a ast::ExprCall,
    keyword_name: &str,
    position: Option<usize>,
) -> Option<(&'a ast::Expr, ruff_text_size::TextRange)> {
    if let Some(position) = position {
        if let Some(arg) = call.arguments.args.get(position) {
            return Some((arg, arg.range()));
        }
    }
    let keyword = call.arguments.keywords.iter().find(|keyword| {
        keyword
            .arg
            .as_ref()
            .is_some_and(|arg| arg.as_str() == keyword_name)
    })?;
    Some((&keyword.value, keyword.value.range()))
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

use ruff_python_ast as ast;
use ruff_text_size::TextRange;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_generics::member_reference_target_bases;
use super::symbol_members::push_member_reference;
use super::symbol_types::type_binding_from_annotation_expr;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn collect_decorator_parameter_references(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
        include_type_surface: bool,
        range: TextRange,
    ) {
        if !include_type_surface {
            return;
        }
        for parameter in function.parameters.iter_non_variadic_params() {
            let Some(annotation) = parameter.annotation() else {
                continue;
            };
            let Some(binding) =
                type_binding_from_annotation_expr(self.module, self.imports, annotation)
            else {
                continue;
            };
            self.collect_parameter_type_surface(owner, &binding, range);
        }
    }

    fn collect_parameter_type_surface(
        &mut self,
        owner: &str,
        binding: &TypeBinding,
        range: TextRange,
    ) {
        let binding = expand_alias_binding(binding, self.available_values);
        for target_base in member_reference_target_bases(&binding) {
            let Some(class_info) = self
                .available_classes
                .iter()
                .find(|class_info| class_info.class == target_base)
            else {
                continue;
            };
            let field_names = class_info
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .chain(class_info.attributes.iter().map(String::as_str))
                .collect::<Vec<_>>();
            for field_name in field_names {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    owner,
                    format!("{target_base}.{field_name}"),
                    AccessKind::Read,
                    range,
                );
            }
        }
    }
}

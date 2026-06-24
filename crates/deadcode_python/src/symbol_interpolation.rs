use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_comprehension_narrowing::apply_comprehension_guard_narrowing;
use super::symbol_expr::target_name;
use super::symbol_iteration::bind_iteration_target;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_list_comprehension_references(
        &mut self,
        owner: &str,
        list_comp: &ast::ExprListComp,
        types: &HashMap<String, TypeBinding>,
    ) {
        let mut scoped_types = types.clone();
        for generator in &list_comp.generators {
            self.collect_expr_references(owner, &generator.iter, &scoped_types);
            if let Some(item_type) = self.iteration_item_type(&generator.iter, &scoped_types) {
                bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
            }
            for guard in &generator.ifs {
                self.collect_expr_references(owner, guard, &scoped_types);
                apply_comprehension_guard_narrowing(guard, &mut scoped_types);
            }
        }
        self.collect_expr_references(owner, &list_comp.elt, &scoped_types);
    }

    pub(super) fn collect_dict_comprehension_references(
        &mut self,
        owner: &str,
        dict_comp: &ast::ExprDictComp,
        types: &HashMap<String, TypeBinding>,
    ) {
        let mut scoped_types = types.clone();
        for generator in &dict_comp.generators {
            self.collect_expr_references(owner, &generator.iter, &scoped_types);
            if let Some(item_type) = self.iteration_item_type(&generator.iter, &scoped_types) {
                bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
            }
            for guard in &generator.ifs {
                self.collect_expr_references(owner, guard, &scoped_types);
                apply_comprehension_guard_narrowing(guard, &mut scoped_types);
            }
        }
        self.collect_expr_references(owner, &dict_comp.key, &scoped_types);
        self.collect_expr_references(owner, &dict_comp.value, &scoped_types);
    }

    pub(super) fn collect_generator_references(
        &mut self,
        owner: &str,
        generator_exp: &ast::ExprGenerator,
        types: &HashMap<String, TypeBinding>,
    ) {
        let mut scoped_types = types.clone();
        for generator in &generator_exp.generators {
            self.collect_expr_references(owner, &generator.iter, &scoped_types);
            if let Some(item_type) = self.iteration_item_type(&generator.iter, &scoped_types) {
                bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
            }
            for guard in &generator.ifs {
                self.collect_expr_references(owner, guard, &scoped_types);
                apply_comprehension_guard_narrowing(guard, &mut scoped_types);
            }
        }
        self.collect_expr_references(owner, &generator_exp.elt, &scoped_types);
    }

    pub(super) fn collect_fstring_references(
        &mut self,
        owner: &str,
        f_string: &ast::ExprFString,
        types: &HashMap<String, TypeBinding>,
    ) {
        for part in f_string.value.iter() {
            self.collect_fstring_part_references(owner, part, types);
        }
    }

    fn collect_fstring_part_references(
        &mut self,
        owner: &str,
        part: &ast::FStringPart,
        types: &HashMap<String, TypeBinding>,
    ) {
        let ast::FStringPart::FString(f_string) = part else {
            return;
        };
        self.collect_interpolations(owner, &f_string.elements, types);
    }

    fn collect_interpolations(
        &mut self,
        owner: &str,
        elements: &ast::InterpolatedStringElements,
        types: &HashMap<String, TypeBinding>,
    ) {
        for interpolation in elements.interpolations() {
            self.collect_expr_references(owner, &interpolation.expression, types);
            if let Some(format_spec) = &interpolation.format_spec {
                self.collect_interpolations(owner, &format_spec.elements, types);
            }
        }
    }
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
    bind_iteration_target(target, item_type, types);
}

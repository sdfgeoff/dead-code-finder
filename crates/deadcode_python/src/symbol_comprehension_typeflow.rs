use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_comprehension_narrowing::apply_comprehension_guard_narrowing;
use super::symbol_expr::target_name;
use super::symbol_iteration::bind_iteration_target;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn list_literal_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::List(list) = expr else {
            return None;
        };
        let item_type = self.list_literal_item_type(&list.elts, types)?;
        Some(TypeBinding {
            base: "list".to_string(),
            args: vec![item_type],
            external: false,
        })
    }

    pub(super) fn list_comprehension_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::ListComp(list_comp) = expr else {
            return None;
        };
        let mut scoped_types = types.clone();
        for generator in &list_comp.generators {
            let item_type = self.iteration_item_type(&generator.iter, &scoped_types)?;
            bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
            for guard in &generator.ifs {
                apply_comprehension_guard_narrowing(guard, &mut scoped_types);
            }
        }
        Some(TypeBinding {
            base: "list".to_string(),
            args: vec![self.expression_flow_binding(&list_comp.elt, &scoped_types)?],
            external: false,
        })
    }

    pub(super) fn dict_comprehension_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::DictComp(dict_comp) = expr else {
            return None;
        };
        let mut scoped_types = types.clone();
        for generator in &dict_comp.generators {
            let item_type = self.iteration_item_type(&generator.iter, &scoped_types)?;
            bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
            for guard in &generator.ifs {
                apply_comprehension_guard_narrowing(guard, &mut scoped_types);
            }
        }
        Some(TypeBinding {
            base: "dict".to_string(),
            args: vec![
                self.expression_flow_binding(&dict_comp.key, &scoped_types)?,
                self.expression_flow_binding(&dict_comp.value, &scoped_types)?,
            ],
            external: false,
        })
    }

    pub(super) fn generator_expression_flow_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Generator(generator_exp) = expr else {
            return None;
        };
        let mut scoped_types = types.clone();
        for generator in &generator_exp.generators {
            let item_type = self.iteration_item_type(&generator.iter, &scoped_types)?;
            bind_comprehension_target(&generator.target, &item_type, &mut scoped_types);
            for guard in &generator.ifs {
                apply_comprehension_guard_narrowing(guard, &mut scoped_types);
            }
        }
        Some(TypeBinding {
            base: "typing.Generator".to_string(),
            args: vec![self.expression_flow_binding(&generator_exp.elt, &scoped_types)?],
            external: false,
        })
    }

    fn list_literal_item_type(
        &self,
        elements: &[ast::Expr],
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let mut item_type = None;
        for element in elements {
            let element_type = self.expression_flow_binding(element, types)?;
            item_type = Some(match item_type {
                None => element_type,
                Some(existing) => merge_item_types(existing, element_type),
            });
        }
        item_type
    }
}

fn merge_item_types(existing: TypeBinding, next: TypeBinding) -> TypeBinding {
    if existing == next {
        return existing;
    }
    let mut args = union_args(existing);
    if !args.iter().any(|arg| arg == &next) {
        args.push(next);
    }
    TypeBinding {
        base: "typing.Union".to_string(),
        args,
        external: false,
    }
}

fn union_args(binding: TypeBinding) -> Vec<TypeBinding> {
    if matches!(binding.base.as_str(), "typing.Union" | "types.UnionType") {
        binding.args
    } else {
        vec![binding]
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

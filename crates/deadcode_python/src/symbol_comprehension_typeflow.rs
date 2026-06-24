use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use super::symbol_iteration::bind_iteration_target;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
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
        }
        Some(TypeBinding {
            base: "list".to_string(),
            args: vec![self.expression_flow_binding(&list_comp.elt, &scoped_types)?],
            external: false,
        })
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

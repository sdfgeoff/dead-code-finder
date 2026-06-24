use std::collections::HashMap;

use ruff_python_ast as ast;

use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn bind_append_receiver_type(
        &self,
        expr: &ast::Expr,
        types: &mut HashMap<String, TypeBinding>,
    ) -> Option<()> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        if attribute.attr.as_str() != "append" {
            return None;
        }
        let ast::Expr::Name(receiver) = attribute.value.as_ref() else {
            return None;
        };
        if types.contains_key(receiver.id.as_str()) {
            return None;
        }
        let item_type = call
            .arguments
            .args
            .first()
            .and_then(|arg| self.assignment_value_binding(arg, types))?;
        types.insert(
            receiver.id.as_str().to_string(),
            TypeBinding {
                base: "list".to_string(),
                args: vec![item_type],
                external: false,
            },
        );
        Some(())
    }
}

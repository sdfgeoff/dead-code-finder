use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_rules::callable_identity;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn json_mapping_call_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        if callable_identity(self.module, self.imports, &call.func).as_deref() == Some("json.loads")
        {
            return Some(json_mapping_type());
        }
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        if attribute.attr.as_str() != "json" {
            return None;
        }
        self.receiver_type_for_expr(&attribute.value, types)
            .filter(|receiver| receiver.external)
            .map(|_| json_mapping_type())
    }
}

fn json_mapping_type() -> TypeBinding {
    TypeBinding {
        base: "dict".to_string(),
        args: vec![
            TypeBinding::erased("str".to_string()),
            TypeBinding::erased("object".to_string()),
        ],
        external: false,
    }
}

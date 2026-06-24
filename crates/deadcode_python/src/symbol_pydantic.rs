use std::collections::HashMap;

use ruff_python_ast as ast;

use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn pydantic_validation_call_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return None;
        };
        let receiver_type = self.receiver_type_for_expr(&attribute.value, types)?;
        match attribute.attr.as_str() {
            "validate_python" if receiver_type.base == "pydantic.TypeAdapter" => receiver_type
                .args
                .first()
                .map(unwrap_annotated_validation_type),
            "model_validate" | "model_validate_json"
                if self.class_derives_from(&receiver_type.base, "pydantic.BaseModel") =>
            {
                Some(receiver_type)
            }
            _ => None,
        }
    }

    fn class_derives_from(&self, concrete_type: &str, base_type: &str) -> bool {
        self.class_derives_from_inner(concrete_type, base_type, &mut Vec::new())
    }

    fn class_derives_from_inner(
        &self,
        concrete_type: &str,
        base_type: &str,
        visited: &mut Vec<String>,
    ) -> bool {
        if visited.iter().any(|visited| visited == concrete_type) {
            return false;
        }
        visited.push(concrete_type.to_string());
        let Some(class_info) = self
            .available_classes
            .iter()
            .find(|class_info| class_info.class == concrete_type)
        else {
            return false;
        };
        class_info.bases.iter().any(|base| {
            base.base == base_type || self.class_derives_from_inner(&base.base, base_type, visited)
        })
    }
}

fn unwrap_annotated_validation_type(binding: &TypeBinding) -> TypeBinding {
    if matches!(
        binding.base.as_str(),
        "typing.Annotated" | "typing_extensions.Annotated"
    ) {
        return binding
            .args
            .first()
            .map(unwrap_annotated_validation_type)
            .unwrap_or_else(|| TypeBinding::erased("object".to_string()));
    }
    TypeBinding {
        base: binding.base.clone(),
        args: binding
            .args
            .iter()
            .map(unwrap_annotated_validation_type)
            .collect(),
        external: binding.external,
    }
}

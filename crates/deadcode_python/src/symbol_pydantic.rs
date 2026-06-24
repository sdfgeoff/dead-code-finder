use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_generics::substitute_type_params;
use super::symbol_members::push_member_reference;
use super::symbol_rules::callable_identity;
use super::symbol_types::{type_binding_from_annotation_expr, type_binding_from_expr};
use super::SymbolCollector;
use deadcode_core::SymbolKind;

use crate::symbol_index::{AccessKind, ClassInfo, FieldAnnotation, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn pydantic_type_adapter_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let callable = callable_identity(self.module, self.imports, &call.func)?;
        if callable != "pydantic.TypeAdapter" {
            return None;
        }
        let target_type = call
            .arguments
            .args
            .first()
            .and_then(|arg| self.type_adapter_target_binding(arg, types))?;
        Some(TypeBinding {
            base: "pydantic.TypeAdapter".to_string(),
            args: vec![target_type],
            external: false,
        })
    }

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

    pub(super) fn collect_pydantic_validation_field_references(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) {
        let ast::Expr::Attribute(attribute) = call.func.as_ref() else {
            return;
        };
        let Some(receiver_type) = self.receiver_type_for_expr(&attribute.value, types) else {
            return;
        };
        if attribute.attr.as_str() == "validate_python"
            && receiver_type.base == "pydantic.TypeAdapter"
        {
            if let Some(target_type) = receiver_type.args.first() {
                self.collect_validated_type_references(
                    owner,
                    target_type,
                    call.range(),
                    &mut Vec::new(),
                );
            }
            return;
        }
        if !is_external_model_validation_call(attribute.attr.as_str(), call) {
            return;
        }
        if !self.class_derives_from(&receiver_type.base, "pydantic.BaseModel") {
            return;
        }
        self.collect_validated_type_references(
            owner,
            &receiver_type,
            call.range(),
            &mut Vec::new(),
        );
    }

    pub(super) fn collect_boundary_function_model_references(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
        range: ruff_text_size::TextRange,
    ) {
        for parameter in function.parameters.iter() {
            let Some(annotation) = parameter.as_parameter().annotation() else {
                continue;
            };
            let Some(binding) =
                type_binding_from_annotation_expr(self.module, self.imports, annotation)
            else {
                continue;
            };
            self.collect_validated_type_references(owner, &binding, range, &mut Vec::new());
        }
        if let Some(returns) = &function.returns {
            if let Some(binding) =
                type_binding_from_annotation_expr(self.module, self.imports, returns)
            {
                self.collect_validated_type_references(owner, &binding, range, &mut Vec::new());
            }
        }
    }

    fn collect_validated_type_references(
        &mut self,
        owner: &str,
        binding: &TypeBinding,
        range: ruff_text_size::TextRange,
        visited: &mut Vec<String>,
    ) {
        let binding = expand_alias_binding(
            &unwrap_annotated_validation_type(binding),
            self.available_values,
        );
        if is_collection_type(&binding.base) {
            for arg in &binding.args {
                self.collect_validated_type_references(owner, arg, range, visited);
            }
            return;
        }
        if self.class_is_enum(&binding.base) {
            for member in self.local_class_attribute_names(&binding.base) {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    owner,
                    format!("{}.{}", binding.base, member),
                    AccessKind::Construct,
                    range,
                );
            }
            return;
        }
        if !self.class_derives_from(&binding.base, "pydantic.BaseModel") {
            return;
        }
        if visited.iter().any(|visited| visited == &binding.base) {
            return;
        }
        visited.push(binding.base.clone());
        let Some(class_info) = self
            .available_classes
            .iter()
            .find(|class_info| class_info.class == binding.base)
            .cloned()
        else {
            return;
        };
        for field in class_fields(self.available_classes, &class_info, &binding) {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{}.{}", class_info.class, field.name),
                AccessKind::Construct,
                range,
            );
            match field.annotation {
                FieldAnnotation::Concrete(annotation) => {
                    self.collect_validated_type_references(owner, &annotation, range, visited);
                }
            }
        }
        visited.pop();
    }

    fn class_derives_from(&self, concrete_type: &str, base_type: &str) -> bool {
        self.class_derives_from_inner(concrete_type, base_type, &mut Vec::new())
    }

    fn type_adapter_target_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        match expr {
            ast::Expr::BinOp(bin_op) if bin_op.op == ast::Operator::BitOr => Some(TypeBinding {
                base: "typing.Union".to_string(),
                args: [
                    self.type_adapter_target_binding(&bin_op.left, types),
                    self.type_adapter_target_binding(&bin_op.right, types),
                ]
                .into_iter()
                .flatten()
                .collect(),
                external: false,
            }),
            ast::Expr::Subscript(subscript) => {
                let base = type_binding_from_expr(self.module, self.imports, &subscript.value)?;
                Some(TypeBinding {
                    external: base.external && !is_typing_container(&base.base),
                    base: base.base,
                    args: self.type_adapter_target_args(&subscript.slice, types),
                })
            }
            ast::Expr::Name(name) => types
                .get(name.id.as_str())
                .and_then(type_object_arg)
                .or_else(|| type_binding_from_expr(self.module, self.imports, expr)),
            _ => type_binding_from_expr(self.module, self.imports, expr),
        }
    }

    fn type_adapter_target_args(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Vec<TypeBinding> {
        match expr {
            ast::Expr::Tuple(tuple) => tuple
                .elts
                .iter()
                .filter_map(|expr| self.type_adapter_target_binding(expr, types))
                .collect(),
            expr => self
                .type_adapter_target_binding(expr, types)
                .into_iter()
                .collect(),
        }
    }

    fn class_is_enum(&self, concrete_type: &str) -> bool {
        ["enum.Enum", "enum.StrEnum", "enum.IntEnum"]
            .iter()
            .any(|base_type| self.class_derives_from(concrete_type, base_type))
    }

    fn local_class_attribute_names(&self, class_name: &str) -> Vec<String> {
        let prefix = format!("{class_name}.");
        self.symbols
            .iter()
            .filter(|symbol| {
                symbol.kind == SymbolKind::Attribute && symbol.qualified_name.starts_with(&prefix)
            })
            .filter_map(|symbol| symbol.qualified_name.strip_prefix(&prefix))
            .map(ToString::to_string)
            .collect()
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

fn class_fields(
    classes: &[ClassInfo],
    class_info: &ClassInfo,
    receiver_type: &TypeBinding,
) -> Vec<crate::symbol_index::ClassFieldInfo> {
    let mut fields = class_info.fields.clone();
    for field in &mut fields {
        match &mut field.annotation {
            FieldAnnotation::Concrete(annotation) => {
                *annotation = substitute_type_params(annotation, class_info, receiver_type);
            }
        }
    }
    for base in &class_info.bases {
        for inherited in class_field_names(classes, &base.base) {
            if !fields.iter().any(|field| field.name == inherited.name) {
                fields.push(inherited);
            }
        }
    }
    fields
}

fn class_field_names(
    classes: &[ClassInfo],
    class_name: &str,
) -> Vec<crate::symbol_index::ClassFieldInfo> {
    let mut fields = Vec::new();
    collect_class_field_names(classes, class_name, &mut fields, &mut Vec::new());
    fields
}

fn collect_class_field_names(
    classes: &[ClassInfo],
    class_name: &str,
    fields: &mut Vec<crate::symbol_index::ClassFieldInfo>,
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
        if !fields.iter().any(|existing| existing.name == field.name) {
            fields.push(field.clone());
        }
    }
    for base in &class_info.bases {
        collect_class_field_names(classes, &base.base, fields, visited);
    }
}

fn is_collection_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "list"
            | "tuple"
            | "set"
            | "frozenset"
            | "typing.List"
            | "typing.Sequence"
            | "collections.abc.Sequence"
            | "typing.Union"
            | "typing.Optional"
            | "Optional"
            | "types.UnionType"
    ) || type_name.ends_with(".Sequence")
        || type_name.ends_with(".Union")
        || type_name.ends_with(".Optional")
}

fn type_object_arg(binding: &TypeBinding) -> Option<TypeBinding> {
    is_type_object(&binding.base)
        .then(|| binding.args.first().cloned())
        .flatten()
}

fn is_type_object(type_name: &str) -> bool {
    matches!(
        type_name,
        "type" | "typing.Type" | "typing_extensions.Type" | "Type"
    ) || type_name.ends_with(".Type")
}

fn is_typing_container(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Annotated"
            | "typing.Callable"
            | "typing.Dict"
            | "typing.FrozenSet"
            | "typing.List"
            | "typing.Optional"
            | "typing.Sequence"
            | "typing.Set"
            | "typing.Tuple"
            | "typing.Type"
            | "typing.Union"
            | "types.UnionType"
    ) || type_name.ends_with(".Annotated")
        || type_name.ends_with(".Optional")
        || type_name.ends_with(".Sequence")
        || type_name.ends_with(".Type")
        || type_name.ends_with(".Union")
}

fn is_external_model_validation_call(method: &str, call: &ast::ExprCall) -> bool {
    match method {
        "model_validate_json" => true,
        "model_validate" => call
            .arguments
            .args
            .first()
            .is_none_or(|argument| !matches!(argument, ast::Expr::Dict(_))),
        _ => false,
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

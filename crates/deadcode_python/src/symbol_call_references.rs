use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_construction::constructed_type_for_call;
use super::symbol_members::push_member_reference;
use super::symbol_rules::{callable_argument_references, callable_identity};
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, CallArgumentType, TypeBinding, UnsupportedExpansion};

impl SymbolCollector<'_> {
    pub(super) fn collect_call_references(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) {
        let is_method_call = if let ast::Expr::Attribute(attribute) = call.func.as_ref() {
            self.collect_member_reference(owner, attribute, AccessKind::Call, types);
            self.collect_typed_dict_get_call_reference(owner, call, types);
            self.collect_expr_references(owner, &attribute.value, types);
            true
        } else {
            self.collect_expr_references(owner, &call.func, types);
            false
        };

        let callee = callable_identity(self.module, self.imports, &call.func);
        let resolved_flow_callee = self
            .resolved_call_target(&call.func, types)
            .or_else(|| callee.clone());
        let flow_callee = resolved_flow_callee.as_deref().map(|callee| {
            self.constructor_init_callee(callee)
                .unwrap_or_else(|| callee.to_string())
        });
        for (name, range) in callable_argument_references(
            self.module,
            self.imports,
            self.rules,
            call,
            callee.as_deref(),
            types,
        ) {
            self.push_reference(owner, &name, range);
        }
        self.collect_factory_model_surfaces(owner, call, types);
        self.collect_pydantic_validation_field_references(owner, call, types);
        self.collect_model_dump_references(owner, call, types);
        for binding in self.local_call_validated_return_bindings(call, types) {
            self.collect_validated_type_references(owner, &binding, call.range(), &mut Vec::new());
        }
        for (position, arg) in call.arguments.args.iter().enumerate() {
            let concrete_types = self.concrete_argument_types(arg, types);
            for concrete_type in concrete_types {
                let Some(callee) = &flow_callee else {
                    continue;
                };
                let position = self.call_argument_position(callee, position, is_method_call);
                self.call_args.push(CallArgumentType {
                    from: owner.to_string(),
                    callee: callee.clone(),
                    position,
                    concrete_type,
                    span: self.locator.span_from_range_string(self.file, arg.range()),
                });
            }
            let lambda_position = flow_callee
                .as_deref()
                .map(|callee| self.call_argument_position(callee, position, is_method_call))
                .unwrap_or(position);
            if !self.collect_callable_lambda_argument(
                owner,
                flow_callee.as_deref(),
                lambda_position,
                arg,
                types,
            ) {
                self.collect_expr_references(owner, arg, types);
            }
        }
        let constructor =
            constructed_type_for_call(self.module, self.imports, self.rules, &call.func, types);
        if let Some((constructor_type, _)) = constructor.as_ref() {
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                owner,
                format!("{constructor_type}.__init__"),
                AccessKind::Construct,
                call.func.range(),
            );
        }
        for keyword in &call.arguments.keywords {
            if self.collect_max_key_lambda_references(owner, call, keyword, types) {
                continue;
            }
            if let (Some(callee), Some(arg)) = (&flow_callee, &keyword.arg) {
                if let Some(position) = self.keyword_argument_position(callee, arg.as_str()) {
                    for concrete_type in self.concrete_argument_types(&keyword.value, types) {
                        self.call_args.push(CallArgumentType {
                            from: owner.to_string(),
                            callee: callee.clone(),
                            position,
                            concrete_type,
                            span: self
                                .locator
                                .span_from_range_string(self.file, keyword.value.range()),
                        });
                    }
                    if self.collect_callable_lambda_argument(
                        owner,
                        Some(callee),
                        position,
                        &keyword.value,
                        types,
                    ) {
                        continue;
                    }
                }
            }
            self.collect_expr_references(owner, &keyword.value, types);
            let Some((constructor_type, is_type_parameter)) = constructor.as_ref() else {
                continue;
            };
            if let Some(arg) = &keyword.arg {
                push_member_reference(
                    self.member_refs,
                    self.locator,
                    self.file,
                    owner,
                    format!("{constructor_type}.{}", arg.as_str()),
                    AccessKind::Construct,
                    keyword.range,
                );
            } else if self.expand_model_dump_keyword(
                owner,
                constructor_type,
                &keyword.value,
                types,
                keyword.range,
            ) {
                continue;
            } else if !is_type_parameter {
                self.unsupported.push(UnsupportedExpansion {
                    from: owner.to_string(),
                    target: constructor_type.clone(),
                    span: self
                        .locator
                        .span_from_range_string(self.file, keyword.range),
                });
            }
        }
    }

    fn collect_callable_lambda_argument(
        &mut self,
        owner: &str,
        callee: Option<&str>,
        position: usize,
        argument: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> bool {
        let ast::Expr::Lambda(lambda) = argument else {
            return false;
        };
        let Some(callee) = callee else {
            return false;
        };
        let Some(annotation) = self
            .available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)
            .and_then(|signature| signature.parameters.get(position))
            .and_then(|parameter| parameter.annotation.as_ref())
        else {
            return false;
        };
        self.collect_callable_argument_lambda_references(owner, lambda, annotation, types)
    }

    fn call_argument_position(&self, callee: &str, position: usize, is_method_call: bool) -> usize {
        if callee.ends_with(".__init__") || (is_method_call && self.has_self_parameter(callee)) {
            position + 1
        } else {
            position
        }
    }

    fn has_self_parameter(&self, callee: &str) -> bool {
        self.available_fn_sigs
            .iter()
            .find(|signature| signature.function == callee)
            .and_then(|signature| signature.parameters.first())
            .is_some_and(|parameter| matches!(parameter.name.as_str(), "self" | "cls"))
    }
}

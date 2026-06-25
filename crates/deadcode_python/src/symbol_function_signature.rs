use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_expr::target_name;
use super::symbol_iteration::bind_collection_unpack_target;
use super::symbol_metadata::function_signature;
use super::symbol_types::type_binding_from_annotation_expr;
use super::SymbolCollector;
use crate::symbol_index::{ResolvedImport, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn push_function_signature(
        &mut self,
        function: &str,
        function_def: &ast::StmtFunctionDef,
        types: &HashMap<String, TypeBinding>,
    ) {
        let mut signature = function_signature(self.module, self.imports, function, function_def);
        let inferred = self.inferred_function_return(function_def, types);
        if let (Some(explicit), Some(inferred)) = (&signature.return_type, &inferred) {
            if explicit.args.is_empty() && self.is_subclass_or_same(&inferred.base, &explicit.base)
            {
                signature.concrete_return_type = Some(inferred.clone());
            }
        }
        signature.return_type = match (signature.return_type, inferred) {
            (None, inferred) => inferred,
            (Some(explicit), Some(inferred))
                if is_tuple_binding(&explicit) && is_tuple_binding(&inferred) =>
            {
                Some(inferred)
            }
            (explicit, _) => explicit,
        };
        self.fn_sigs.push(signature);
    }

    pub(super) fn collect_function_annotation_references(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
    ) {
        let types = HashMap::new();
        for parameter in function.parameters.iter() {
            if let Some(annotation) = parameter.as_parameter().annotation() {
                self.collect_expr_references(owner, annotation, &types);
            }
        }
        if let Some(returns) = &function.returns {
            self.collect_expr_references(owner, returns, &types);
        }
    }

    pub(super) fn function_type_bindings(
        &self,
        function: &ast::StmtFunctionDef,
        class_name: Option<&str>,
        module_types: &HashMap<String, TypeBinding>,
    ) -> HashMap<String, TypeBinding> {
        let mut types = HashMap::new();
        if let Some(class_name) = class_name {
            types.insert(
                "self".to_string(),
                TypeBinding::erased(format!("{}.{}", self.module, class_name)),
            );
            types.insert(
                "cls".to_string(),
                TypeBinding::erased(format!("{}.{}", self.module, class_name)),
            );
        }
        for parameter in function.parameters.iter() {
            let parameter = parameter.as_parameter();
            if let Some(annotation) = parameter.annotation() {
                if let Some(type_name) = type_binding_from_annotation(
                    self.module,
                    self.imports,
                    annotation,
                    module_types,
                ) {
                    types.insert(parameter.name.as_str().to_string(), type_name);
                }
            }
        }
        types
    }

    fn inferred_function_return(
        &self,
        function_def: &ast::StmtFunctionDef,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let mut inferred = None;
        let mut scoped_types = types.clone();
        for statement in &function_def.body {
            match statement {
                ast::Stmt::Assign(assign) => {
                    let Some(binding) = self.assignment_value_binding(&assign.value, &scoped_types)
                    else {
                        continue;
                    };
                    for target in &assign.targets {
                        if let Some(name) = target_name(target) {
                            scoped_types.insert(name.to_string(), binding.clone());
                        } else {
                            bind_collection_unpack_target(target, &binding, &mut scoped_types);
                        }
                    }
                }
                ast::Stmt::AnnAssign(assign) => {
                    let Some(name) = target_name(&assign.target) else {
                        continue;
                    };
                    let Some(annotation) = type_binding_from_annotation_expr(
                        self.module,
                        self.imports,
                        &assign.annotation,
                    ) else {
                        continue;
                    };
                    scoped_types.insert(name.to_string(), annotation);
                }
                ast::Stmt::Return(return_stmt) => {
                    let Some(value) = &return_stmt.value else {
                        return None;
                    };
                    let binding = self.assignment_value_binding(value, &scoped_types)?;
                    if inferred
                        .as_ref()
                        .is_some_and(|existing: &TypeBinding| existing != &binding)
                    {
                        return None;
                    }
                    inferred = Some(binding);
                }
                _ => {}
            }
        }
        inferred
    }
}

fn is_tuple_binding(binding: &TypeBinding) -> bool {
    matches!(binding.base.as_str(), "tuple" | "typing.Tuple" | "Tuple")
        || binding.base.ends_with(".tuple")
}

fn type_binding_from_annotation(
    module: &str,
    imports: &[ResolvedImport],
    annotation: &ast::Expr,
    module_types: &HashMap<String, TypeBinding>,
) -> Option<TypeBinding> {
    if let ast::Expr::Name(name) = annotation {
        if let Some(binding) = module_types.get(name.id.as_str()) {
            return Some(binding.clone());
        }
    }
    type_binding_from_annotation_expr(module, imports, annotation)
}

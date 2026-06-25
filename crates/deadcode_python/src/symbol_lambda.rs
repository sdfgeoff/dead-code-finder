use std::collections::HashMap;

use ruff_python_ast as ast;

use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_callable_argument_lambda_references(
        &mut self,
        owner: &str,
        lambda: &ast::ExprLambda,
        callable_annotation: &TypeBinding,
        types: &HashMap<String, TypeBinding>,
    ) -> bool {
        if !is_callable_type(&callable_annotation.base) {
            return false;
        }
        let Some(parameters) = lambda.parameters.as_deref() else {
            return false;
        };
        let mut scoped_types = types.clone();
        let parameter_types = callable_annotation.args.split_last().map(|(_, args)| args);
        let Some(parameter_types) = parameter_types else {
            return false;
        };
        for (parameter, parameter_type) in parameters.iter().zip(parameter_types) {
            scoped_types.insert(
                parameter.as_parameter().name.as_str().to_string(),
                parameter_type.clone(),
            );
        }
        self.collect_expr_references(owner, &lambda.body, &scoped_types);
        true
    }

    pub(super) fn collect_lambda_references(
        &mut self,
        owner: &str,
        lambda: &ast::ExprLambda,
        types: &HashMap<String, TypeBinding>,
    ) {
        if lambda
            .parameters
            .as_deref()
            .is_some_and(|parameters| parameters.iter().next().is_some())
        {
            return;
        }
        let mut scoped_types = types.clone();
        if let Some(parameters) = lambda.parameters.as_deref() {
            for parameter in parameters.iter() {
                scoped_types.remove(parameter.as_parameter().name.as_str());
            }
        }
        self.collect_expr_references(owner, &lambda.body, &scoped_types);
    }

    pub(super) fn collect_max_key_lambda_references(
        &mut self,
        owner: &str,
        call: &ast::ExprCall,
        keyword: &ast::Keyword,
        types: &HashMap<String, TypeBinding>,
    ) -> bool {
        if !matches!(call.func.as_ref(), ast::Expr::Name(name) if name.id.as_str() == "max")
            || !keyword
                .arg
                .as_ref()
                .is_some_and(|arg| arg.as_str() == "key")
        {
            return false;
        }
        let ast::Expr::Lambda(lambda) = &keyword.value else {
            return false;
        };
        let Some(item_type) = call
            .arguments
            .args
            .first()
            .and_then(|arg| self.iteration_item_type(arg, types))
        else {
            return false;
        };
        let Some(parameter) = lambda
            .parameters
            .as_deref()
            .and_then(|parameters| parameters.iter().next())
        else {
            return false;
        };
        let mut scoped_types = types.clone();
        scoped_types.insert(
            parameter.as_parameter().name.as_str().to_string(),
            item_type,
        );
        self.collect_expr_references(owner, &lambda.body, &scoped_types);
        true
    }
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    )
}

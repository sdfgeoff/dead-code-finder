use std::collections::HashMap;

use ruff_python_ast as ast;

use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
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

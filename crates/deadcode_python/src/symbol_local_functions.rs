use std::collections::HashMap;

use deadcode_core::SymbolKind;
use ruff_python_ast as ast;

use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn collect_local_function_references(
        &mut self,
        owner: &str,
        function: &ast::StmtFunctionDef,
        types: &mut HashMap<String, TypeBinding>,
    ) {
        let function_owner = format!("{owner}.{}", function.name.as_str());
        self.push_symbol(
            function_owner.clone(),
            function.name.as_str(),
            SymbolKind::Function,
            function.range,
        );
        types.insert(
            function.name.to_string(),
            function_object_binding(function_owner.clone()),
        );

        let mut function_types = types.clone();
        function_types.extend(self.function_type_bindings(function, None, types));
        self.push_function_signature(&function_owner, function, &function_types);
        self.collect_function_references(&function_owner, function, function_types);
    }
}

fn function_object_binding(function_owner: String) -> TypeBinding {
    TypeBinding {
        base: function_owner,
        args: Vec::new(),
        external: false,
    }
}

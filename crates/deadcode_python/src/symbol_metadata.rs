use ruff_python_ast as ast;

use crate::symbol_index::{ClassInfo, FunctionSignature, ResolvedImport};

use super::symbol_types::type_name_from_expr;

pub(super) fn class_info(
    module: &str,
    imports: &[ResolvedImport],
    class: String,
    class_def: &ast::StmtClassDef,
) -> ClassInfo {
    let bases = class_def
        .arguments
        .as_ref()
        .map(|arguments| {
            arguments
                .args
                .iter()
                .filter_map(|base| type_name_from_expr(module, imports, base))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ClassInfo { class, bases }
}

pub(super) fn function_signature(
    module: &str,
    imports: &[ResolvedImport],
    function: &str,
    function_def: &ast::StmtFunctionDef,
) -> FunctionSignature {
    let parameter_types = function_def
        .parameters
        .iter()
        .map(|parameter| {
            parameter
                .as_parameter()
                .annotation()
                .and_then(|annotation| type_name_from_expr(module, imports, annotation))
        })
        .collect();
    FunctionSignature {
        function: function.to_string(),
        parameter_types,
    }
}

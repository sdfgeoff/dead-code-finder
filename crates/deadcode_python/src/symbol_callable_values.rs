use std::collections::HashMap;

use ruff_python_ast as ast;

use super::symbol_aliases::expand_alias_binding;
use super::symbol_rules::callable_identity;
use super::SymbolCollector;
use crate::symbol_index::{ImportTarget, ResolvedImport, TypeBinding, ValueBinding};

impl SymbolCollector<'_> {
    pub(super) fn callable_value_return_binding(
        &self,
        call: &ast::ExprCall,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        let binding = self.callable_value_binding(&call.func, types)?;
        is_callable_type(&binding.base)
            .then(|| binding.args.last().cloned())
            .flatten()
    }

    fn callable_value_binding(
        &self,
        expr: &ast::Expr,
        types: &HashMap<String, TypeBinding>,
    ) -> Option<TypeBinding> {
        match expr {
            ast::Expr::Name(name) => types.get(name.id.as_str()).cloned().or_else(|| {
                imported_module_value_binding(
                    self.module,
                    self.imports,
                    name.id.as_str(),
                    self.available_values,
                )
            }),
            ast::Expr::Attribute(_) => {
                callable_identity(self.module, self.imports, expr).and_then(|qualified| {
                    self.available_values
                        .iter()
                        .find(|value| value.qualified_name == qualified)
                        .map(|value| value.binding.clone())
                })
            }
            ast::Expr::Subscript(subscript) => self.callable_value_binding(&subscript.value, types),
            _ => None,
        }
        .map(|binding| expand_alias_binding(&binding, self.available_values))
    }
}

fn imported_module_value_binding(
    current_module: &str,
    imports: &[ResolvedImport],
    name: &str,
    values: &[ValueBinding],
) -> Option<TypeBinding> {
    let imported = imports.iter().find_map(|import| match &import.target {
        ImportTarget::Symbol {
            module,
            name: imported_name,
            external: false,
        } if import.binding == name => Some(format!("{module}.{imported_name}")),
        ImportTarget::Module {
            module: imported_module,
            external: false,
        } if import.binding == name && imported_module == &format!("{current_module}.{name}") => {
            Some(imported_module.clone())
        }
        _ => None,
    })?;
    values
        .iter()
        .find(|value| value.qualified_name == imported)
        .map(|value| value.binding.clone())
}

fn is_callable_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Callable" | "collections.abc.Callable" | "Callable"
    ) || type_name.ends_with(".Callable")
}

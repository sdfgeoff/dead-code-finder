use std::collections::HashMap;

use super::symbol_aliases::expand_alias_binding;
use super::SymbolCollector;
use crate::symbol_index::{ImportTarget, TypeBinding, ValueBinding};

impl SymbolCollector<'_> {
    pub(super) fn push_imported_value_bindings(
        &mut self,
        types: &mut HashMap<String, TypeBinding>,
        import_start: usize,
    ) {
        let imports = &self.imports[import_start..];
        for import in imports {
            let Some(qualified_name) = import_value_qualified_name(&import.target) else {
                continue;
            };
            let Some(binding) = self
                .available_values
                .iter()
                .find(|value| value.qualified_name == qualified_name)
                .map(|value| expand_alias_binding(&value.binding, self.available_values))
            else {
                continue;
            };
            types.insert(import.binding.clone(), binding);
        }
    }

    pub(super) fn push_value_binding(&mut self, name: &str, binding: TypeBinding) {
        let qualified_name = format!("{}.{}", self.module, name);
        if let Some(existing) = self
            .value_bindings
            .iter_mut()
            .find(|value| value.qualified_name == qualified_name)
        {
            existing.binding = binding;
            return;
        }
        self.value_bindings.push(ValueBinding {
            qualified_name,
            binding,
        });
    }
}

fn import_value_qualified_name(target: &ImportTarget) -> Option<String> {
    match target {
        ImportTarget::Symbol {
            module,
            name,
            external: false,
        } => Some(format!("{module}.{name}")),
        _ => None,
    }
}

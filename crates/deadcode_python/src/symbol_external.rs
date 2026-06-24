use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn mark_external_if_outside_project(&self, binding: &mut TypeBinding) {
        if is_type_container(&binding.base) {
            binding.external = false;
            for arg in &mut binding.args {
                self.mark_external_if_outside_project(arg);
            }
            return;
        }
        if binding.external || self.is_project_type(&binding.base) {
            return;
        }
        binding.external = true;
    }

    fn is_project_type(&self, type_name: &str) -> bool {
        self.known_modules
            .iter()
            .any(|module| type_name == module || type_name.starts_with(&format!("{module}.")))
    }
}

fn is_type_container(type_name: &str) -> bool {
    matches!(
        type_name,
        "typing.Optional"
            | "Optional"
            | "typing.Union"
            | "types.UnionType"
            | "typing.List"
            | "typing.Dict"
            | "typing.Mapping"
            | "typing.Sequence"
            | "typing.Set"
            | "typing.Tuple"
            | "list"
            | "dict"
            | "set"
            | "tuple"
    )
}

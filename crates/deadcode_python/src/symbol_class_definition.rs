use std::collections::HashMap;

use ruff_python_ast as ast;
use ruff_text_size::Ranged;

use super::symbol_members::push_member_reference;
use super::symbol_types::type_binding_from_expr;
use super::SymbolCollector;
use crate::symbol_index::{AccessKind, TypeBinding};

impl SymbolCollector<'_> {
    pub(super) fn collect_class_definition_references(
        &mut self,
        class_owner: &str,
        class_def: &ast::StmtClassDef,
        module_types: &HashMap<String, TypeBinding>,
    ) {
        let Some(arguments) = &class_def.arguments else {
            return;
        };
        for base in &arguments.args {
            self.collect_expr_references(class_owner, base, module_types);
            let Some(binding) = type_binding_from_expr(self.module, self.imports, base) else {
                continue;
            };
            if binding.external {
                continue;
            }
            push_member_reference(
                self.member_refs,
                self.locator,
                self.file,
                class_owner,
                format!("{}.__init_subclass__", binding.base),
                AccessKind::Call,
                base.range(),
            );
        }
    }
}

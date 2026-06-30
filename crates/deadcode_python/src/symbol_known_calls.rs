use ruff_python_ast as ast;

use super::symbol_rules::callable_identity;
use super::SymbolCollector;
use crate::symbol_index::TypeBinding;

impl SymbolCollector<'_> {
    pub(super) fn known_call_result_binding(&self, expr: &ast::Expr) -> Option<TypeBinding> {
        let ast::Expr::Call(call) = expr else {
            return None;
        };
        let callable = callable_identity(self.module, self.imports, &call.func)?;
        let base = match callable.as_str() {
            "datetime.datetime.now"
            | "datetime.datetime.utcnow"
            | "datetime.datetime.fromtimestamp"
            | "datetime.datetime.strptime"
            | "datetime.datetime.combine" => "datetime.datetime",
            "datetime.date.today"
            | "datetime.date.fromtimestamp"
            | "datetime.date.fromisoformat" => "datetime.date",
            "pathlib.Path" => "pathlib.Path",
            "inspect.stack" => {
                return Some(TypeBinding {
                    base: "list".to_string(),
                    args: vec![TypeBinding {
                        base: "inspect.FrameInfo".to_string(),
                        args: Vec::new(),
                        external: true,
                    }],
                    external: false,
                });
            }
            _ => return None,
        };
        Some(TypeBinding {
            base: base.to_string(),
            args: Vec::new(),
            external: true,
        })
    }
}

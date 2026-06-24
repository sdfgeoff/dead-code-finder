use ruff_python_ast as ast;

pub(super) fn is_main_guard(if_stmt: &ast::StmtIf) -> bool {
    let ast::Expr::Compare(compare) = if_stmt.test.as_ref() else {
        return false;
    };
    if !matches!(compare.left.as_ref(), ast::Expr::Name(name) if name.id.as_str() == "__name__") {
        return false;
    }
    if compare.ops.as_ref() != [ast::CmpOp::Eq] {
        return false;
    }
    matches!(
        compare.comparators.as_ref(),
        [ast::Expr::StringLiteral(value)] if value.value.to_str() == "__main__"
    )
}

pub(super) fn target_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Name(name) => Some(name.id.as_str()),
        _ => None,
    }
}

pub(super) fn self_attribute_name(expr: &ast::Expr) -> Option<&str> {
    match expr {
        ast::Expr::Attribute(attribute) => match attribute.value.as_ref() {
            ast::Expr::Name(name) if name.id.as_str() == "self" => Some(attribute.attr.as_str()),
            _ => None,
        },
        _ => None,
    }
}

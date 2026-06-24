use ruff_text_size::TextRange;

use super::super::{AccessKind, MemberReference, SourceLocator};

pub(super) fn push_member_reference(
    member_references: &mut Vec<MemberReference>,
    locator: &SourceLocator,
    file: &str,
    owner: &str,
    target: String,
    access: AccessKind,
    range: TextRange,
) {
    member_references.push(MemberReference {
        from: owner.to_string(),
        target,
        access,
        span: locator.span_from_range_string(file, range),
    });
}

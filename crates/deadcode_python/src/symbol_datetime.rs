use crate::symbol_index::TypeBinding;

pub(super) fn is_datetime_like(binding: &TypeBinding) -> bool {
    matches!(binding.base.as_str(), "datetime.datetime" | "datetime.date")
}

pub(super) fn is_timedelta(binding: &TypeBinding) -> bool {
    binding.base == "datetime.timedelta"
}

use std::path::Path;

use deadcode_core::SourceSpan;
use ruff_text_size::TextRange;

pub(crate) struct SourceLocator {
    line_starts: Vec<usize>,
}

impl SourceLocator {
    pub(crate) fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    pub(crate) fn span(&self, file: &Path, range: TextRange) -> SourceSpan {
        self.span_from_range_string(&file.display().to_string(), range)
    }

    pub(crate) fn span_from_range_string(&self, file: &str, range: TextRange) -> SourceSpan {
        let offset = range.start().to_usize();
        let line_index = self.line_starts.partition_point(|start| *start <= offset) - 1;
        SourceSpan::new(
            file,
            line_index + 1,
            offset - self.line_starts[line_index] + 1,
        )
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use crate::symbol_index::SymbolIndexError;

pub(super) fn collect_python_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), SymbolIndexError> {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "py") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|source| SymbolIndexError::ReadDirectory {
        path: path.to_path_buf(),
        source,
    })?;

    for entry in entries {
        let entry = entry.map_err(|source| SymbolIndexError::ReadDirectory {
            path: path.to_path_buf(),
            source,
        })?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            collect_python_files(&entry_path, files)?;
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "py")
        {
            files.push(entry_path);
        }
    }

    Ok(())
}

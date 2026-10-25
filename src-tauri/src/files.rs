use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn allowed_extension(file_type: &str) -> bool {
    matches!(file_type, "pptx" | "docx" | "xlsx" | "pdf")
}

pub fn safe_file_name(name: &str, file_type: &str) -> io::Result<String> {
    if !allowed_extension(file_type) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsupported deliverable type",
        ));
    }
    let extension = Path::new(name)
        .extension()
        .and_then(|item| item.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case(file_type) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file extension does not match deliverable type",
        ));
    }
    let safe: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    if safe.starts_with('.') || safe.contains("..") || safe.len() > 180 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsafe output file name",
        ));
    }
    Ok(safe)
}

pub fn validate_nonempty_report(path: &Path, file_type: &str) -> io::Result<u64> {
    if !allowed_extension(file_type) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsupported report type",
        ));
    }
    if path
        .extension()
        .and_then(|item| item.to_str())
        .map(|item| item.eq_ignore_ascii_case(file_type))
        != Some(true)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output extension mismatch",
        ));
    }
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "generated report is empty",
        ));
    }
    Ok(metadata.len())
}

pub fn relative_display_path(root: &Path, path: &Path) -> io::Result<String> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "path outside report root"))
}

pub fn canonical_inside(root: &Path, candidate: &Path) -> io::Result<PathBuf> {
    let root = root.canonicalize()?;
    let candidate = candidate.canonicalize()?;
    if !candidate.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path outside approved root",
        ));
    }
    Ok(candidate)
}

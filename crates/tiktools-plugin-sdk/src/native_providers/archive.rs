use super::*;

/// Extract an npm tarball (`package/...` entries) into memory. Rejects
/// absolute paths, `..` segments, symlinks, hardlinks, non-regular
/// files, and anything outside `package/`; enforces file-count and
/// byte caps. Never writes: callers decide what lands on disk.
pub(super) fn extract_npm_tarball(
    package: &str,
    tarball: &[u8],
) -> Result<Vec<ExtractedFile>, NativeProviderError> {
    let tar_error = |message: String| NativeProviderError::Tar {
        package: package.to_owned(),
        message,
    };
    let gzip = flate2::read::GzDecoder::new(tarball);
    let mut archive = tar::Archive::new(gzip);
    let entries = archive
        .entries()
        .map_err(|error| tar_error(format!("unreadable archive: {error}")))?;
    let mut files = Vec::new();
    let mut total: u64 = 0;
    for entry in entries {
        let mut entry = entry.map_err(|error| tar_error(format!("unreadable entry: {error}")))?;
        if files.len() >= MAX_EXTRACTED_FILES {
            return Err(tar_error(format!(
                "archive holds more than {MAX_EXTRACTED_FILES} files"
            )));
        }
        if !entry.header().entry_type().is_file() {
            return Err(tar_error("archive holds a non-regular file".to_owned()));
        }
        let path: PathBuf = entry
            .path()
            .map_err(|error| tar_error(format!("undecodable entry path: {error}")))?
            .into_owned();
        let mut components = path.components();
        if !matches!(components.next(), Some(Component::Normal(first)) if first == "package") {
            return Err(tar_error(format!(
                "entry {} escapes the package root",
                path.display()
            )));
        }
        let mut relative = PathBuf::new();
        for component in components {
            let Some(name) = (match component {
                Component::Normal(name) => Some(name),
                _ => None,
            }) else {
                return Err(tar_error(format!(
                    "entry {} is not a safe relative path",
                    path.display()
                )));
            };
            relative.push(name);
        }
        if relative.as_os_str().is_empty() {
            return Err(tar_error("archive holds an empty entry path".to_owned()));
        }
        let relative = relative.display().to_string().replace('\\', "/");
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|error| tar_error(format!("unreadable entry {relative}: {error}")))?;
        total += bytes.len() as u64;
        if total > MAX_EXTRACTED_BYTES {
            return Err(tar_error(format!(
                "archive extracts to more than {MAX_EXTRACTED_BYTES} bytes"
            )));
        }
        files.push(ExtractedFile { relative, bytes });
    }
    if files.is_empty() {
        return Err(tar_error("archive holds no files".to_owned()));
    }
    Ok(files)
}

/// One extracted npm package file: path relative to the package root
/// plus its bytes.
pub(super) struct ExtractedFile {
    pub(super) relative: String,
    pub(super) bytes: Vec<u8>,
}

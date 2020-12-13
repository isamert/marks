pub mod file_utils {
    use walkdir::{DirEntry};

    /// Returns if the file starts with dot (".") or not.
    pub fn is_hidden(entry: &DirEntry) -> bool {
        entry.file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    }
}

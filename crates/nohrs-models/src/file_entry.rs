use serde::Serialize;

/// A filesystem entry as produced by the services layer and consumed by the UI.
///
/// Field types are intentionally primitive (`String`, `u64`) so this type stays
/// free of any toolkit or service dependency and can live at the bottom of the
/// dependency graph in `nohrs-models`.
#[derive(Debug, Serialize, Clone)]
pub struct FileEntryDto {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub size: u64,
    pub modified: u64,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub enum FileKind {
    File,
    Dir,
    Link,
}

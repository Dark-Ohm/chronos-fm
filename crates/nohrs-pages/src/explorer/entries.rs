use super::types::SortKey;
use nohrs_services::fs::listing::FileEntryDto;

pub fn sort_entries(entries: &mut [FileEntryDto], key: SortKey, asc: bool) {
    entries.sort_by(|a, b| {
        // Directories before files
        let is_dir_a = a.kind == "dir";
        let is_dir_b = b.kind == "dir";

        match is_dir_b.cmp(&is_dir_a) {
            std::cmp::Ordering::Equal => {
                let order = match key {
                    SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    SortKey::Size => a.size.cmp(&b.size),
                    SortKey::Modified => a.modified.cmp(&b.modified),
                    SortKey::Type => {
                        let ext_a = get_extension(&a.name, &a.kind);
                        let ext_b = get_extension(&b.name, &b.kind);
                        ext_a.cmp(&ext_b)
                    }
                };
                if asc {
                    order
                } else {
                    order.reverse()
                }
            }
            kind_order => kind_order,
        }
    });
}

pub fn get_extension(name: &str, kind: &str) -> String {
    match kind {
        "dir" => "0_dir".to_string(),
        "file" => std::path::Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_else(|| "zzz_noext".to_string()),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: &str, size: u64) -> FileEntryDto {
        FileEntryDto {
            name: name.to_string(),
            path: format!("/{name}"),
            kind: kind.to_string(),
            size,
            modified: 0,
        }
    }

    #[test]
    fn sort_entries_keeps_directories_before_files() {
        let mut entries = vec![
            entry("b.txt", "file", 1),
            entry("alpha", "dir", 0),
            entry("a.txt", "file", 2),
        ];
        sort_entries(&mut entries, SortKey::Name, true);
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["alpha", "a.txt", "b.txt"]);
    }

    #[test]
    fn sort_entries_reverses_within_each_group_when_descending() {
        let mut entries = vec![
            entry("a.txt", "file", 1),
            entry("b.txt", "file", 2),
            entry("dir-a", "dir", 0),
        ];
        sort_entries(&mut entries, SortKey::Name, false);
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        // Directories still sort ahead of files; ordering within files reverses.
        assert_eq!(names, ["dir-a", "b.txt", "a.txt"]);
    }

    #[test]
    fn get_extension_classifies_kinds() {
        assert_eq!(get_extension("anything", "dir"), "0_dir");
        assert_eq!(get_extension("Main.RS", "file"), "rs");
        assert_eq!(get_extension("README", "file"), "zzz_noext");
    }
}

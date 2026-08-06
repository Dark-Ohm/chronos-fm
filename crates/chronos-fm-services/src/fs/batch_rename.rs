//! Batch-rename pattern machinery: a tiny mini-DSL for rendering new file names
//! from placeholders, plus pure preview computation with collision resolution.
//! No disk *mutation* here — renames live in `ops`; the only I/O is a read-only
//! existence check against the destination directory for collision resolution.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::listing::FileEntryDto;
use super::ops;

/// Status of one previewed rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameStatus {
    /// The target is free and will be applied as computed.
    Ok,
    /// The computed target collided (with another batch target or an existing
    /// file on disk) and was auto-resolved to a unique name.
    ResolvedCollision,
}

/// One row of the live preview: what a file is now and what it will become.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenamePreview {
    /// The current file name.
    pub old_name: String,
    /// The name it will be renamed to (already collision-resolved).
    pub new_name: String,
    /// Whether the target needed collision resolution.
    pub status: RenameStatus,
}

/// Parse errors for the pattern mini-DSL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    /// An unknown placeholder such as `{q}`.
    UnknownPlaceholder(String),
    /// A `{` without a matching `}`, or a stray `}`.
    UnclosedBrace,
    /// `{n:W}` with a non-numeric width.
    InvalidWidth(String),
}

/// A parsed batch-rename pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Literal(String),
    Name,
    Ext,
    Counter { width: Option<usize> },
}

/// Splits `name` into (stem, extension) by the decided last-extension rule
/// (spec §3): the extension is the final dot-suffix, the stem is everything up
/// to the **first** dot — so `archive.tar.gz` → (`archive`, `gz`). Names with
/// no dot (or a leading dot, e.g. `.hidden`) yield an empty extension.
pub fn split_stem_ext(name: &str) -> (String, String) {
    if let Some((head, _)) = name.split_once('.') {
        if !head.is_empty() {
            if let Some((_, tail)) = name.rsplit_once('.') {
                if !tail.is_empty() {
                    return (head.to_string(), tail.to_string());
                }
            }
        }
    }
    (name.to_string(), String::new())
}

/// Applies a find/replace to `stem`; an empty `find` is a no-op.
pub fn apply_find_replace(stem: &str, find: &str, replace: &str) -> String {
    if find.is_empty() {
        stem.to_string()
    } else {
        stem.replace(find, replace)
    }
}

impl Template {
    /// Parses the pattern mini-DSL. Placeholders: `{name}`, `{ext}`, `{n}`,
    /// `{n:W}` (zero-padded counter to width `W`). Literal braces are
    /// `{{`/`}}`. Unknown placeholders and malformed braces are errors — never
    /// silently mis-rename (the dialog disables Apply on `Err`).
    pub fn parse(pattern: &str) -> Result<Template, PatternError> {
        let mut tokens = Vec::new();
        let mut literal = String::new();
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '{' if chars.get(i + 1) == Some(&'{') => {
                    literal.push('{');
                    i += 2;
                }
                '}' if chars.get(i + 1) == Some(&'}') => {
                    literal.push('}');
                    i += 2;
                }
                '{' => {
                    let Some(rel) = chars[i + 1..].iter().position(|&c| c == '}') else {
                        return Err(PatternError::UnclosedBrace);
                    };
                    let close = i + 1 + rel;
                    let body: String = chars[i + 1..close].iter().collect();
                    if !literal.is_empty() {
                        tokens.push(Token::Literal(std::mem::take(&mut literal)));
                    }
                    tokens.push(match body.as_str() {
                        "name" => Token::Name,
                        "ext" => Token::Ext,
                        "n" => Token::Counter { width: None },
                        _ if body.starts_with("n:") => {
                            let width: usize = body[2..]
                                .parse()
                                .map_err(|_| PatternError::InvalidWidth(body[2..].to_string()))?;
                            Token::Counter { width: Some(width) }
                        }
                        _ => return Err(PatternError::UnknownPlaceholder(body)),
                    });
                    i = close + 1;
                }
                '}' => return Err(PatternError::UnclosedBrace),
                c => {
                    literal.push(c);
                    i += 1;
                }
            }
        }
        if !literal.is_empty() {
            tokens.push(Token::Literal(literal));
        }
        Ok(Template { tokens })
    }

    /// Renders one new file name from the parsed tokens.
    pub fn render(&self, stem: &str, ext: &str, counter: u64) -> String {
        let mut out = String::new();
        for token in &self.tokens {
            match token {
                Token::Literal(s) => out.push_str(s),
                Token::Name => out.push_str(stem),
                Token::Ext => out.push_str(ext),
                Token::Counter { width } => match width {
                    Some(w) => out.push_str(&format!("{counter:0w$}")),
                    None => out.push_str(&counter.to_string()),
                },
            }
        }
        out
    }
}

/// Returns the first `name (2).ext`, `name (3).ext`, … that is free both within
/// the batch's already-taken targets and on disk in the entry's directory
/// (mirrors `ops::unique_name`'s naming scheme, extended across both scopes).
fn resolve_collision(candidate: &str, entry_path: &str, taken: &HashSet<String>) -> String {
    let parent = Path::new(entry_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let (stem, ext) = split_stem_ext(candidate);
    let mut n = 2u64;
    loop {
        let bumped = if ext.is_empty() {
            format!("{stem} ({n})")
        } else {
            format!("{stem} ({n}).{ext}")
        };
        if !taken.contains(&bumped) && !ops::would_conflict(&parent.join(&bumped)) {
            return bumped;
        }
        n += 1;
    }
}

/// Computes the full preview for a batch in selection order. Targets are
/// resolved against the batch's own earlier targets and against existing files
/// on disk (only for names that actually change), so two files can never land
/// on the same name; any resolution is surfaced via `RenameStatus`.
pub fn build_preview(
    entries: &[FileEntryDto],
    template: &Template,
    find: &str,
    replace: &str,
    start: u64,
) -> Vec<RenamePreview> {
    let mut previews = Vec::with_capacity(entries.len());
    let mut taken: HashSet<String> = HashSet::new();

    for (ix, entry) in entries.iter().enumerate() {
        let (stem, ext) = split_stem_ext(&entry.name);
        let stem = apply_find_replace(&stem, find, replace);
        let mut candidate = template.render(&stem, &ext, start + ix as u64);
        let mut status = RenameStatus::Ok;

        if taken.contains(&candidate) {
            candidate = resolve_collision(&candidate, &entry.path, &taken);
            status = RenameStatus::ResolvedCollision;
        } else if candidate != entry.name {
            let parent = Path::new(&entry.path)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            if ops::would_conflict(&parent.join(&candidate)) {
                candidate = resolve_collision(&candidate, &entry.path, &taken);
                status = RenameStatus::ResolvedCollision;
            }
        }

        taken.insert(candidate.clone());
        previews.push(RenamePreview {
            old_name: entry.name.clone(),
            new_name: candidate,
            status,
        });
    }
    previews
}

#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]

    use super::*;
    use chronos_fm_models::file_entry::FileEntryDto;
    use std::fs;
    use tempfile::tempdir;

    fn entry(dir: &std::path::Path, name: &str) -> FileEntryDto {
        FileEntryDto {
            name: name.to_string(),
            path: dir.join(name).to_string_lossy().to_string(),
            kind: "file".to_string(),
            size: 0,
            modified: 0,
        }
    }

    #[test]
    fn parse_accepts_all_placeholders_and_literals() {
        let template = Template::parse("{name}_{n:3}.{ext}").expect("parses");
        assert_eq!(template.render("photo", "jpg", 7), "photo_007.jpg");
    }

    #[test]
    fn parse_handles_escaped_braces() {
        let template = Template::parse("{{lit}}").expect("parses");
        assert_eq!(template.render("stem", "ext", 1), "{lit}");
    }

    #[test]
    fn parse_rejects_unknown_placeholder() {
        assert_eq!(
            Template::parse("{q}"),
            Err(PatternError::UnknownPlaceholder("q".to_string()))
        );
    }

    #[test]
    fn parse_rejects_unclosed_and_stray_braces() {
        assert_eq!(Template::parse("a{name"), Err(PatternError::UnclosedBrace));
        assert_eq!(Template::parse("a}"), Err(PatternError::UnclosedBrace));
    }

    #[test]
    fn parse_rejects_bad_width() {
        assert_eq!(
            Template::parse("{n:x}"),
            Err(PatternError::InvalidWidth("x".to_string()))
        );
    }

    #[test]
    fn split_stem_ext_last_extension_rule() {
        assert_eq!(split_stem_ext("archive.tar.gz"), ("archive".into(), "gz".into()));
        assert_eq!(split_stem_ext("noext"), ("noext".into(), String::new()));
        assert_eq!(split_stem_ext(".hidden"), (".hidden".into(), String::new()));
    }

    #[test]
    fn apply_find_replace_basic() {
        assert_eq!(apply_find_replace("stem", "", "x"), "stem");
        assert_eq!(apply_find_replace("foobar", "foo", "bar"), "barbar");
        assert_eq!(apply_find_replace("stem", "nomatch", "x"), "stem");
    }

    #[test]
    fn build_preview_numbers_from_custom_start_with_padding() {
        let dir = tempdir().unwrap();
        for name in ["a.txt", "b.txt", "c.txt"] {
            fs::write(dir.path().join(name), "x").unwrap();
        }
        let entries: Vec<FileEntryDto> = ["a.txt", "b.txt", "c.txt"]
            .iter()
            .map(|n| entry(dir.path(), n))
            .collect();
        let template = Template::parse("{n:3}.{ext}").unwrap();
        let previews = build_preview(&entries, &template, "", "", 5);
        let names: Vec<&str> = previews.iter().map(|p| p.new_name.as_str()).collect();
        assert_eq!(names, vec!["005.txt", "006.txt", "007.txt"]);
        assert!(previews.iter().all(|p| p.status == RenameStatus::Ok));
    }

    #[test]
    fn build_preview_resolves_batch_internal_collision() {
        let dir = tempdir().unwrap();
        for name in ["a.txt", "b.txt"] {
            fs::write(dir.path().join(name), "x").unwrap();
        }
        let entries: Vec<FileEntryDto> = ["a.txt", "b.txt"]
            .iter()
            .map(|n| entry(dir.path(), n))
            .collect();
        // Both files render `x.{ext}` -> both want `x.txt`.
        let template = Template::parse("x.{ext}").unwrap();
        let previews = build_preview(&entries, &template, "", "", 1);
        assert_eq!(previews[0].new_name, "x.txt");
        assert_eq!(previews[0].status, RenameStatus::Ok);
        assert_eq!(previews[1].new_name, "x (2).txt");
        assert_eq!(previews[1].status, RenameStatus::ResolvedCollision);
    }

    #[test]
    fn build_preview_resolves_disk_collision() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "x").unwrap();
        // Pre-existing `b.txt` on disk: `b.{ext}` over `a.txt` collides.
        fs::write(dir.path().join("b.txt"), "occupied").unwrap();
        let entries = vec![entry(dir.path(), "a.txt")];
        let template = Template::parse("b.{ext}").unwrap();
        let previews = build_preview(&entries, &template, "", "", 1);
        assert_eq!(previews[0].new_name, "b (2).txt");
        assert_eq!(previews[0].status, RenameStatus::ResolvedCollision);

        // A same-name no-op stays Ok (no spurious resolution).
        let entries = vec![entry(dir.path(), "b.txt")];
        let previews = build_preview(&entries, &template, "", "", 1);
        assert_eq!(previews[0].new_name, "b.txt");
        assert_eq!(previews[0].status, RenameStatus::Ok);
    }
}

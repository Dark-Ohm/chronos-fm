//! Terminal emulator resolution and launch for "Open Terminal Here" (T055).
//!
//! Pure argv-array construction — deliberately **not** the `mime.rs`
//! `sh -c` string-building pattern (T055 Must #4). The child is spawned
//! detached (no `wait()`), so it reparents to init on exit; a zombie window
//! is an accepted v1 tradeoff (documented in the brief), not a reason to
//! reintroduce a shell.

use std::path::Path;
use std::process::{Command, Stdio};

/// Ordered fallback list used when `$TERMINAL` is unset or invalid (T055 Must
/// #2).
pub const CANDIDATES: [&str; 5] = ["kitty", "alacritty", "foot", "gnome-terminal", "xterm"];

/// Whether a terminal spec is a single binary name/path. `$TERMINAL` is
/// treated as a bare executable only: a value that looks like a command line
/// (contains whitespace) is invalid by design — it is never split-and-argv'd,
/// because that would guess at shell-quoting semantics (T055 stamp 5).
pub fn is_valid_terminal_spec(spec: &str) -> bool {
    !spec.is_empty() && !spec.chars().any(char::is_whitespace)
}

/// Pure resolution: choose the terminal to launch given a PATH predicate.
///
/// `$TERMINAL` wins when set and single-token **and** available; otherwise
/// the first [`CANDIDATES`] entry the predicate accepts. The predicate is
/// injected so the ordering logic is testable without spawning anything
/// (T055 tests — pure resolution-order).
pub fn resolve_terminal(
    terminal_env: Option<&str>,
    path_predicate: impl Fn(&str) -> bool,
) -> Option<String> {
    if let Some(spec) = terminal_env {
        if is_valid_terminal_spec(spec) && path_predicate(spec) {
            return Some(spec.to_owned());
        }
    }
    CANDIDATES
        .iter()
        .copied()
        .find(|candidate| path_predicate(candidate))
        .map(str::to_owned)
}

/// Builds the detached launch command: the bare binary with `current_dir`
/// set to `cwd` and all stdio nulled. Never a shell string — the cwd travels
/// via `current_dir`, not via an interpolated argument.
pub fn build_command(bin: &str, cwd: &Path) -> Command {
    let mut command = Command::new(bin);
    command
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// PATH-lookup predicate for a bare binary name or absolute path.
pub fn binary_on_path(bin: &str) -> bool {
    let path = Path::new(bin);
    if path.is_absolute() {
        path.is_file()
    } else {
        std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file())
        })
    }
}

/// Resolves and spawns a terminal rooted at `cwd` (T055).
///
/// Resolution order: `$TERMINAL` (single token) → [`CANDIDATES`]. Failures
/// name the command that failed (T055 Must #3). The child is spawned
/// detached and left running; it reparents to init on exit.
pub fn open_terminal_here(cwd: &str) -> Result<(), String> {
    let cwd_path = Path::new(cwd);
    if !cwd_path.is_dir() {
        return Err(format!("cannot open terminal in {cwd}: not a directory"));
    }
    let Some(bin) = resolve_terminal(std::env::var("TERMINAL").ok().as_deref(), binary_on_path)
    else {
        return Err(
            "no terminal found: set $TERMINAL or install kitty/alacritty/foot/gnome-terminal/xterm"
                .to_string(),
        );
    };
    tracing::info!("launching terminal {bin} in {cwd}");
    build_command(&bin, cwd_path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Failed to launch {bin}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // -- Pure resolution order --

    #[test]
    fn unset_terminal_uses_first_candidate_on_path() {
        let present = |bin: &str| matches!(bin, "kitty" | "foot");
        assert_eq!(
            resolve_terminal(None, present),
            Some("kitty".to_string()),
            "first PATH-present candidate wins"
        );
    }

    #[test]
    fn valid_single_token_terminal_env_wins() {
        let present = |bin: &str| bin == "foot" || bin == "my-term";
        assert_eq!(
            resolve_terminal(Some("my-term"), present),
            Some("my-term".to_string()),
            "$TERMINAL beats the candidate list"
        );
    }

    #[test]
    fn terminal_env_not_on_path_falls_back_to_candidates() {
        let present = |bin: &str| bin == "alacritty";
        assert_eq!(
            resolve_terminal(Some("ghost-terminal"), present),
            Some("alacritty".to_string()),
            "an unavailable $TERMINAL falls through"
        );
    }

    #[test]
    fn multi_token_terminal_env_is_invalid_and_falls_back() {
        let present = |bin: &str| bin == "kitty";
        // "kitty --single-instance" looks like a command line: invalid, never
        // split-and-argv'd.
        assert_eq!(
            resolve_terminal(Some("kitty --single-instance"), present),
            Some("kitty".to_string())
        );
    }

    #[test]
    fn no_candidate_available_resolves_to_none() {
        let present = |_bin: &str| false;
        assert_eq!(resolve_terminal(None, present), None);
        assert_eq!(resolve_terminal(Some("term --flag"), present), None);
    }

    #[test]
    fn spec_validity_rejects_whitespace_and_empty() {
        assert!(is_valid_terminal_spec("kitty"));
        assert!(is_valid_terminal_spec("/usr/bin/foot"));
        assert!(!is_valid_terminal_spec("kitty --single-instance"));
        assert!(!is_valid_terminal_spec(""));
        assert!(!is_valid_terminal_spec("  "));
    }

    // -- Argv construction (the mime.rs `sh -c` regression guard) --

    #[test]
    fn build_command_is_bare_argv_with_no_shell_string() {
        let dir = tempdir().unwrap();
        let command = build_command("kitty", dir.path());
        assert_eq!(command.get_program(), "kitty", "the binary is the program");
        assert_eq!(
            command.get_args().count(),
            0,
            "no args at all — the cwd must not be an interpolated argument"
        );
    }

    #[test]
    fn build_command_sets_cwd_via_current_dir() {
        // Real end-to-end check of `current_dir` semantics without a shell:
        // spawn `pwd` in the built command's directory and read its output.
        let dir = tempdir().unwrap();
        let mut command = build_command("pwd", dir.path());
        command.stdout(Stdio::piped()); // re-enable stdout only for the probe
        let output = command.output().expect("pwd spawns");
        assert!(output.status.success());
        let cwd = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let expected = dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| dir.path().to_path_buf());
        assert_eq!(cwd, expected.to_string_lossy());
    }

    #[test]
    fn binary_on_path_resolves_through_path_environment() {
        // A binary that exists somewhere on this machine's PATH must resolve.
        let probe = if Path::new("/bin/sh").exists() {
            "sh"
        } else {
            "true"
        };
        assert!(binary_on_path(probe), "{probe} should be on PATH");
        assert!(!binary_on_path("definitely-not-a-real-binary-xyz"));
    }

    #[test]
    fn open_terminal_here_rejects_non_directory_cwd() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("note.txt");
        fs::write(&file, "x").unwrap();
        let error = open_terminal_here(file.to_str().unwrap()).unwrap_err();
        assert!(error.contains("not a directory"), "{error}");
    }
}

//! Port of `server/platform/detect.py` to Rust.
//!
//! Mirrors `get_platform`, `_resolve_python`, `build_venv_env`, `build_command`,
//! and `is_compatible` so spawned commands match the Python behaviour byte-for-byte.

use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::{collections::HashMap, env, path::PathBuf};

const PYTHON_NAMES: &[&str] = &["python", "python3", "py"];

pub static PLATFORM: Lazy<&'static str> = Lazy::new(detect_platform_inner);

fn detect_platform_inner() -> &'static str {
    if cfg!(target_os = "windows") {
        return "windows";
    }
    if let Ok(text) = std::fs::read_to_string("/proc/version") {
        if text.to_lowercase().contains("microsoft") {
            return "wsl2";
        }
    }
    "linux"
}

pub fn get_platform() -> &'static str {
    *PLATFORM
}

pub fn is_compatible(entity_platform: &str) -> bool {
    if entity_platform == "all" {
        return true;
    }
    let plat = get_platform();
    match entity_platform {
        "windows" => plat == "windows",
        "linux" => plat == "linux" || plat == "wsl2",
        _ => true,
    }
}

/// Resolve `python`, `python3`, or `py` to an absolute interpreter path.
/// Non-Python commands are returned unchanged.
///
/// 1. If `venv_path` is given, use `<venv>/Scripts/python.exe` (Windows) or
///    `<venv>/bin/python` (Unix) when it exists.
/// 2. Fallback to env var `PYTHON_FALLBACK`, then `which::which("python")` /
///    `python3` / `py`.
pub fn resolve_python(name: &str, venv_path: Option<&str>) -> String {
    if !PYTHON_NAMES.contains(&name) {
        return name.to_string();
    }

    if let Some(vp) = venv_path {
        let venv = PathBuf::from(vp);
        let candidate = if get_platform() == "windows" {
            venv.join("Scripts").join("python.exe")
        } else {
            venv.join("bin").join("python")
        };
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }

    if let Ok(v) = env::var("PYTHON_FALLBACK") {
        if !v.is_empty() {
            return v;
        }
    }

    for candidate in ["python", "python3", "py"] {
        if let Ok(p) = which::which(candidate) {
            return p.to_string_lossy().to_string();
        }
    }

    name.to_string()
}

/// Build the process environment, prepending `<venv>/(bin|Scripts)` to PATH and
/// setting `VIRTUAL_ENV`. Returns `None` when neither `venv_path` nor `env`
/// alter the current environment.
pub fn build_venv_env(
    venv_path: Option<&str>,
    extra_env: Option<&HashMap<String, String>>,
) -> Option<HashMap<String, String>> {
    if venv_path.is_none() && extra_env.is_none() {
        return None;
    }

    let mut merged: HashMap<String, String> = env::vars().collect();
    if let Some(e) = extra_env {
        for (k, v) in e {
            merged.insert(k.clone(), v.clone());
        }
    }

    if let Some(vp) = venv_path {
        let venv = PathBuf::from(vp);
        let bin_dir = if get_platform() == "windows" {
            venv.join("Scripts")
        } else {
            venv.join("bin")
        };
        merged.insert("VIRTUAL_ENV".into(), venv.to_string_lossy().to_string());
        let path_sep = if cfg!(target_os = "windows") {
            ";"
        } else {
            ":"
        };
        let existing_path = merged.remove("PATH").unwrap_or_default();
        merged.insert(
            "PATH".into(),
            format!("{}{}{}", bin_dir.to_string_lossy(), path_sep, existing_path),
        );
    }

    Some(merged)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    Exec,
    Shell,
}

impl RunMode {
    pub fn parse(s: &str) -> Self {
        match s {
            "shell" => RunMode::Shell,
            _ => RunMode::Exec,
        }
    }
}

/// Build the argv list for `tokio::process::Command::new(args[0]).args(&args[1..])`.
/// Mirrors `server/platform/detect.py::build_command`.
pub fn build_command(
    command: &str,
    args: &[String],
    platform: Option<&str>,
    venv_path: Option<&str>,
    run_mode: RunMode,
) -> Result<Vec<String>> {
    let plat = platform.unwrap_or_else(|| get_platform());
    let is_windows = plat == "windows";

    if run_mode == RunMode::Shell {
        let mut line = command.to_string();
        if !args.is_empty() {
            if is_windows {
                line = format!("{} {}", line, list2cmdline(args));
            } else {
                let quoted = args
                    .iter()
                    .map(|a| posix_quote(a))
                    .collect::<Vec<_>>()
                    .join(" ");
                line = format!("{} {}", line, quoted);
            }
        }
        if is_windows {
            return Ok(vec![
                "cmd.exe".into(),
                "/d".into(),
                "/s".into(),
                "/c".into(),
                line,
            ]);
        }
        return Ok(vec!["bash".into(), "-lc".into(), line]);
    }

    let parts = if is_windows {
        split_winshell(command)
    } else {
        shlex::split(command).ok_or_else(|| anyhow!("Failed to parse command: {}", command))?
    };
    if parts.is_empty() {
        return Err(anyhow!("Command cannot be empty"));
    }

    let mut head = parts.clone();
    let extra: Vec<String> = args.to_vec();

    if is_windows {
        // Python's endswith is case-sensitive — mirror that.
        if head[0].ends_with(".bat") || head[0].ends_with(".cmd") {
            let mut out = vec!["cmd.exe".into(), "/c".into()];
            out.extend(head);
            out.extend(extra);
            return Ok(out);
        }
        if PYTHON_NAMES.contains(&head[0].as_str()) {
            head[0] = resolve_python(&head[0], venv_path);
            let mut out = head;
            out.extend(extra);
            return Ok(out);
        }
        // Resolve .cmd/.exe via PATH (case-sensitive ext match)
        if let Ok(resolved) = which::which(&head[0]) {
            let s = resolved.to_string_lossy().to_string();
            if s.ends_with(".cmd") || s.ends_with(".bat") {
                let mut out = vec!["cmd.exe".into(), "/c".into(), s];
                out.extend(head.into_iter().skip(1));
                out.extend(extra);
                return Ok(out);
            }
            head[0] = s;
        }
        let mut out = head;
        out.extend(extra);
        return Ok(out);
    }

    // linux / wsl2
    if head[0].ends_with(".sh") {
        let mut out = vec!["bash".into()];
        out.extend(head);
        out.extend(extra);
        return Ok(out);
    }
    head[0] = resolve_python(&head[0], venv_path);
    let mut out = head;
    out.extend(extra);
    Ok(out)
}

// --- helpers ---

fn posix_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".into();
    }
    let safe = s.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, '@' | '%' | '_' | '-' | '+' | '=' | ':' | ',' | '.' | '/')
    });
    if safe {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\"'\"'"))
    }
}

// ---------------------------------------------------------------------------
// Rust runtime support (S9)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct RustSpec {
    pub manifest_path: Option<String>,
    pub binary_path: Option<String>,
    pub cargo_profile: Option<String>,
    pub cargo_features: Option<String>,
    pub prebuild: bool,
}

/// Resolve `cargo` via `CARGO_BIN` env override, then PATH.
pub fn resolve_cargo() -> Result<String> {
    if let Ok(v) = env::var("CARGO_BIN") {
        if !v.is_empty() {
            return Ok(v);
        }
    }
    which::which("cargo")
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|_| anyhow!("`cargo` not found in PATH (set CARGO_BIN to override)"))
}

/// Build the argv for the actual long-running process for a Rust service.
///
/// - If `binary_path` is set and exists, exec it directly with `args`.
/// - Otherwise, exec `cargo run --manifest-path <path> --profile <profile>
///   [--features <features>] -- <args>`.
pub fn build_rust_argv(spec: &RustSpec, args: &[String]) -> Result<Vec<String>> {
    if let Some(bin) = &spec.binary_path {
        if !bin.is_empty() && std::path::Path::new(bin).exists() {
            let mut out = vec![bin.clone()];
            out.extend_from_slice(args);
            return Ok(out);
        }
    }
    let manifest = spec
        .manifest_path
        .as_ref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Rust service requires manifest_path or an existing binary_path"))?;
    let cargo = resolve_cargo()?;
    let profile = spec
        .cargo_profile
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "release".into());
    let mut out = vec![
        cargo,
        "run".into(),
        "--manifest-path".into(),
        manifest.clone(),
        "--profile".into(),
        profile,
    ];
    if let Some(features) = spec.cargo_features.as_ref().filter(|s| !s.is_empty()) {
        out.push("--features".into());
        out.push(features.clone());
    }
    out.push("--".into());
    out.extend_from_slice(args);
    Ok(out)
}

/// Build the argv for `cargo build` when `prebuild=true`. Returns `None` if
/// prebuild is disabled or no manifest is configured (binary-only service).
pub fn build_rust_prebuild_argv(spec: &RustSpec) -> Result<Option<Vec<String>>> {
    if !spec.prebuild {
        return Ok(None);
    }
    let manifest = match spec.manifest_path.as_ref().filter(|s| !s.is_empty()) {
        Some(m) => m,
        None => return Ok(None),
    };
    let cargo = resolve_cargo()?;
    let profile = spec
        .cargo_profile
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "release".into());
    let mut out = vec![
        cargo,
        "build".into(),
        "--manifest-path".into(),
        manifest.clone(),
        "--profile".into(),
        profile,
    ];
    if let Some(features) = spec.cargo_features.as_ref().filter(|s| !s.is_empty()) {
        out.push("--features".into());
        out.push(features.clone());
    }
    Ok(Some(out))
}

/// Mirror Python's `subprocess.list2cmdline` for Windows argv joining.
/// Quotes args containing spaces, escapes embedded double-quotes.
fn list2cmdline(args: &[String]) -> String {
    let mut out = String::new();
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let needs_quotes =
            arg.is_empty() || arg.contains(' ') || arg.contains('\t') || arg.contains('"');
        if !needs_quotes {
            out.push_str(arg);
            continue;
        }
        out.push('"');
        let mut backslashes = 0usize;
        for c in arg.chars() {
            if c == '\\' {
                backslashes += 1;
            } else if c == '"' {
                for _ in 0..(backslashes * 2 + 1) {
                    out.push('\\');
                }
                backslashes = 0;
                out.push('"');
            } else {
                for _ in 0..backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push(c);
            }
        }
        for _ in 0..(backslashes * 2) {
            out.push('\\');
        }
        out.push('"');
    }
    out
}

/// Tokenize a Windows command line in posix=False mode (Python `shlex(posix=False)`).
/// Quote characters are **preserved verbatim** in tokens; they only affect whether
/// whitespace splits a token. Backslashes are literal.
fn split_winshell(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut in_squote = false;
    let mut in_dquote = false;
    let mut has_token = false;
    for ch in s.chars() {
        if in_squote {
            buf.push(ch);
            if ch == '\'' {
                in_squote = false;
            }
        } else if in_dquote {
            buf.push(ch);
            if ch == '"' {
                in_dquote = false;
            }
        } else if ch == '\'' {
            buf.push(ch);
            in_squote = true;
            has_token = true;
        } else if ch == '"' {
            buf.push(ch);
            in_dquote = true;
            has_token = true;
        } else if ch.is_whitespace() {
            if has_token {
                out.push(std::mem::take(&mut buf));
                has_token = false;
            }
        } else {
            buf.push(ch);
            has_token = true;
        }
    }
    if has_token {
        out.push(buf);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn linux_simple() {
        let r = build_command("echo hello", &[], Some("linux"), None, RunMode::Exec).unwrap();
        assert_eq!(r, s(&["echo", "hello"]));
    }

    #[test]
    fn linux_with_args() {
        let r = build_command("node", &s(&["app.js"]), Some("linux"), None, RunMode::Exec).unwrap();
        assert_eq!(r, s(&["node", "app.js"]));
    }

    #[test]
    fn linux_sh_script() {
        let r =
            build_command("./run.sh", &s(&["arg"]), Some("linux"), None, RunMode::Exec).unwrap();
        assert_eq!(r, s(&["bash", "./run.sh", "arg"]));
    }

    #[test]
    fn linux_shell_mode() {
        let r = build_command("ls | grep foo", &[], Some("linux"), None, RunMode::Shell).unwrap();
        assert_eq!(r, s(&["bash", "-lc", "ls | grep foo"]));
    }

    #[test]
    fn linux_shell_with_args_quotes_them() {
        let r = build_command(
            "echo",
            &s(&["hello world", "$VAR"]),
            Some("linux"),
            None,
            RunMode::Shell,
        )
        .unwrap();
        assert_eq!(r, s(&["bash", "-lc", "echo 'hello world' '$VAR'"]));
    }

    #[test]
    fn windows_bat() {
        let r = build_command(
            "script.bat",
            &s(&["arg"]),
            Some("windows"),
            None,
            RunMode::Exec,
        )
        .unwrap();
        assert_eq!(r, s(&["cmd.exe", "/c", "script.bat", "arg"]));
    }

    #[test]
    fn windows_cmd_uppercase_ext_not_wrapped() {
        // Python's `endswith` is case-sensitive. ".CMD" (uppercase) does NOT match ".cmd",
        // so the command is treated as a generic exe (no `cmd.exe /c` wrapping).
        let r = build_command("Run.CMD", &[], Some("windows"), None, RunMode::Exec).unwrap();
        assert_eq!(r, s(&["Run.CMD"]));
    }

    #[test]
    fn windows_shell_mode() {
        let r = build_command(
            "dir & echo done",
            &[],
            Some("windows"),
            None,
            RunMode::Shell,
        )
        .unwrap();
        assert_eq!(r, s(&["cmd.exe", "/d", "/s", "/c", "dir & echo done"]));
    }

    #[test]
    fn windows_shell_with_args_uses_list2cmdline() {
        let r = build_command(
            "echo",
            &s(&["hello world"]),
            Some("windows"),
            None,
            RunMode::Shell,
        )
        .unwrap();
        assert_eq!(r, s(&["cmd.exe", "/d", "/s", "/c", "echo \"hello world\""]));
    }

    #[test]
    fn empty_command_errors() {
        let err = build_command("", &[], Some("linux"), None, RunMode::Exec).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn list2cmdline_no_quotes_for_simple() {
        assert_eq!(list2cmdline(&s(&["foo", "bar"])), "foo bar");
    }

    #[test]
    fn list2cmdline_quotes_spaces() {
        assert_eq!(list2cmdline(&s(&["a b", "c"])), "\"a b\" c");
    }

    #[test]
    fn list2cmdline_escapes_quotes() {
        assert_eq!(
            list2cmdline(&s(&["he said \"hi\""])),
            "\"he said \\\"hi\\\"\""
        );
    }

    #[test]
    fn list2cmdline_quotes_empty() {
        assert_eq!(list2cmdline(&s(&[""])), "\"\"");
    }

    #[test]
    fn split_winshell_quoted_preserves_quotes() {
        // shlex(posix=False) keeps the quote characters in the token.
        assert_eq!(
            split_winshell(r#"python "my script.py" arg"#),
            s(&["python", "\"my script.py\"", "arg"])
        );
    }

    #[test]
    fn split_winshell_backslashes_preserved() {
        // Unlike posix shlex, backslashes are literal on Windows
        assert_eq!(
            split_winshell(r"C:\Python\python.exe script.py"),
            s(&[r"C:\Python\python.exe", "script.py"])
        );
    }

    #[test]
    fn is_compatible_all() {
        assert!(is_compatible("all"));
    }

    #[test]
    fn is_compatible_windows() {
        let expected = get_platform() == "windows";
        assert_eq!(is_compatible("windows"), expected);
    }

    #[test]
    fn is_compatible_linux_matches_wsl2() {
        // when running on wsl2, linux should still be compatible
        let plat = get_platform();
        let expected = plat == "linux" || plat == "wsl2";
        assert_eq!(is_compatible("linux"), expected);
    }

    #[test]
    fn resolve_python_passthrough_non_python() {
        assert_eq!(resolve_python("npm", None), "npm");
        assert_eq!(resolve_python("./tool", None), "./tool");
    }

    #[test]
    fn build_venv_env_none_no_inputs() {
        assert!(build_venv_env(None, None).is_none());
    }

    #[test]
    fn build_venv_env_sets_virtual_env() {
        let env = build_venv_env(Some("/tmp/venv"), None).unwrap();
        assert_eq!(env.get("VIRTUAL_ENV").unwrap(), "/tmp/venv");
        let path = env.get("PATH").unwrap();
        assert!(
            path.starts_with("/tmp/venv"),
            "PATH should start with venv root: {}",
            path
        );
    }

    #[test]
    fn build_venv_env_merges_extra() {
        let mut extra = HashMap::new();
        extra.insert("FOO".into(), "bar".into());
        let env = build_venv_env(None, Some(&extra)).unwrap();
        assert_eq!(env.get("FOO").unwrap(), "bar");
    }

    #[test]
    fn run_mode_default_exec() {
        assert_eq!(RunMode::parse("exec"), RunMode::Exec);
        assert_eq!(RunMode::parse(""), RunMode::Exec);
        assert_eq!(RunMode::parse("shell"), RunMode::Shell);
    }

    #[test]
    fn linux_python_with_venv_resolves() {
        // resolve_python keys off host get_platform() (matches Python parity);
        // venv layout differs per OS, so the bin/python candidate is only checked
        // on linux/wsl2 hosts.
        if get_platform() == "windows" {
            return;
        }
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let py = bin.join("python");
        std::fs::write(&py, "").unwrap();
        let venv = tmp.path().to_string_lossy().to_string();
        let r = build_command(
            "python",
            &s(&["app.py"]),
            Some("linux"),
            Some(&venv),
            RunMode::Exec,
        )
        .unwrap();
        assert_eq!(r[0], py.to_string_lossy());
        assert_eq!(r[1], "app.py");
    }

    #[test]
    fn windows_python_with_venv_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        let scripts = tmp.path().join("Scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        let py = scripts.join("python.exe");
        std::fs::write(&py, "").unwrap();
        // resolve_python keys off cfg(target_os) under the hood via get_platform();
        // on non-Windows hosts the Scripts/python.exe candidate is rejected, so this
        // test only meaningfully asserts on Windows.
        if get_platform() != "windows" {
            return;
        }
        let venv = tmp.path().to_string_lossy().to_string();
        let r = build_command(
            "python",
            &s(&["x.py"]),
            Some("windows"),
            Some(&venv),
            RunMode::Exec,
        )
        .unwrap();
        assert_eq!(r[0], py.to_string_lossy());
    }

    #[test]
    fn resolve_python_venv_missing_falls_through() {
        // venv dir does not exist → falls back to PATH lookup or name passthrough.
        let r = resolve_python("python", Some("/definitely/not/a/venv"));
        // Either an absolute resolved path (which::which) or the literal "python" when
        // not found. Just verify it didn't return the venv candidate.
        assert!(!r.contains("/definitely/not/a/venv"));
    }

    #[test]
    fn build_venv_env_windows_path_separator() {
        let env = build_venv_env(Some("C:/tmp/venv"), None).unwrap();
        let path = env.get("PATH").unwrap();
        let sep = if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        };
        assert!(path.contains(sep), "PATH missing separator: {}", path);
    }

    #[test]
    fn linux_extra_args_appended_after_split() {
        let r = build_command(
            "python -u",
            &s(&["script.py", "--flag"]),
            Some("linux"),
            None,
            RunMode::Exec,
        )
        .unwrap();
        // first element is resolved python; check tail
        assert_eq!(&r[r.len() - 3..], &s(&["-u", "script.py", "--flag"])[..]);
    }

    #[test]
    fn windows_quoted_script_path_preserves_quotes() {
        let r = build_command(
            r#"python "C:\Path With Space\app.py""#,
            &[],
            Some("windows"),
            None,
            RunMode::Exec,
        )
        .unwrap();
        // shlex(posix=False) keeps the quotes on the token; python parity
        assert_eq!(r[1], "\"C:\\Path With Space\\app.py\"");
    }

    #[test]
    fn linux_shell_args_empty_string_quoted() {
        let r = build_command("echo", &s(&[""]), Some("linux"), None, RunMode::Shell).unwrap();
        assert_eq!(r, s(&["bash", "-lc", "echo ''"]));
    }

    #[test]
    fn windows_shell_no_args_no_trailing_space() {
        let r = build_command("dir", &[], Some("windows"), None, RunMode::Shell).unwrap();
        assert_eq!(r, s(&["cmd.exe", "/d", "/s", "/c", "dir"]));
    }

    #[test]
    fn windows_bat_no_args() {
        let r = build_command("run.bat", &[], Some("windows"), None, RunMode::Exec).unwrap();
        assert_eq!(r, s(&["cmd.exe", "/c", "run.bat"]));
    }

    #[test]
    fn linux_sh_no_extra_args() {
        let r = build_command("./go.sh", &[], Some("linux"), None, RunMode::Exec).unwrap();
        assert_eq!(r, s(&["bash", "./go.sh"]));
    }

    #[test]
    fn posix_quote_safe_chars_unquoted() {
        assert_eq!(posix_quote("hello"), "hello");
        assert_eq!(posix_quote("a-b_c.d/e:f"), "a-b_c.d/e:f");
    }

    #[test]
    fn posix_quote_escapes_inner_quote() {
        assert_eq!(posix_quote("it's"), "'it'\"'\"'s'");
    }

    // --- Rust runtime ---

    #[test]
    fn rust_argv_with_existing_binary_skips_cargo() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("app.exe");
        std::fs::write(&bin, "").unwrap();
        let spec = RustSpec {
            binary_path: Some(bin.to_string_lossy().to_string()),
            ..Default::default()
        };
        let r = build_rust_argv(&spec, &s(&["--port", "8080"])).unwrap();
        assert_eq!(r[0], bin.to_string_lossy());
        assert_eq!(&r[1..], &["--port", "8080"]);
    }

    #[test]
    fn rust_argv_missing_binary_falls_back_to_cargo_run() {
        let spec = RustSpec {
            manifest_path: Some("/proj/Cargo.toml".into()),
            binary_path: Some("/nope".into()),
            cargo_profile: Some("dev".into()),
            cargo_features: Some("a,b".into()),
            prebuild: false,
        };
        let r = build_rust_argv(&spec, &s(&["--flag"]));
        // resolve_cargo may fail in stripped envs; only assert structure when found
        if let Ok(argv) = r {
            assert_eq!(argv[1], "run");
            assert!(argv.iter().any(|x| x == "--manifest-path"));
            assert!(argv.iter().any(|x| x == "/proj/Cargo.toml"));
            assert!(argv.iter().any(|x| x == "--profile"));
            assert!(argv.iter().any(|x| x == "dev"));
            assert!(argv.iter().any(|x| x == "--features"));
            assert!(argv.iter().any(|x| x == "a,b"));
            assert!(argv.contains(&"--".to_string()));
            assert_eq!(argv.last().unwrap(), "--flag");
        }
    }

    #[test]
    fn rust_argv_no_manifest_no_binary_errors() {
        let spec = RustSpec::default();
        let err = build_rust_argv(&spec, &[]).unwrap_err();
        assert!(err.to_string().contains("manifest_path"), "got: {err}");
    }

    #[test]
    fn rust_prebuild_disabled_returns_none() {
        let spec = RustSpec {
            manifest_path: Some("/proj/Cargo.toml".into()),
            prebuild: false,
            ..Default::default()
        };
        assert!(build_rust_prebuild_argv(&spec).unwrap().is_none());
    }

    #[test]
    fn rust_prebuild_returns_cargo_build_argv() {
        let spec = RustSpec {
            manifest_path: Some("/proj/Cargo.toml".into()),
            prebuild: true,
            cargo_profile: Some("release".into()),
            ..Default::default()
        };
        if let Ok(Some(argv)) = build_rust_prebuild_argv(&spec) {
            assert_eq!(argv[1], "build");
            assert!(argv.iter().any(|x| x == "--manifest-path"));
            assert!(argv.iter().any(|x| x == "/proj/Cargo.toml"));
        }
    }
}

"""Dump build_command outputs (with inputs) for Rust port cross-check.

Cases avoid python/which resolution because PATH differs between Python's
`shutil.which` and Rust's `which::which` on the same Windows host. Resolution
behavior is covered by dedicated Rust unit tests with tempdir venvs.

Usage: py tools/dump_build_commands.py > tests/fixtures/build_commands.json
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from server.platform.detect import build_command  # type: ignore


CASES = [
    # (name, command, args, platform, venv_path, run_mode)
    ("linux_simple", "echo hello", [], "linux", None, "exec"),
    ("linux_node_with_args", "node", ["app.js"], "linux", None, "exec"),
    ("linux_sh_script", "./run.sh", ["arg"], "linux", None, "exec"),
    ("linux_sh_no_args", "./go.sh", [], "linux", None, "exec"),
    ("linux_node_no_args", "node", [], "linux", None, "exec"),
    ("linux_shell_pipe", "ls | grep foo", [], "linux", None, "shell"),
    ("linux_shell_with_args", "echo", ["hello world", "$VAR"], "linux", None, "shell"),
    ("linux_shell_empty_arg", "echo", [""], "linux", None, "shell"),
    ("linux_shell_no_args", "uptime", [], "linux", None, "shell"),
    ("linux_shell_quote_single", "echo", ["it's"], "linux", None, "shell"),
    ("linux_multi_args", "node", ["app.js", "--port", "3000"], "linux", None, "exec"),
    ("windows_bat", "script.bat", ["arg"], "windows", None, "exec"),
    ("windows_bat_no_args", "run.bat", [], "windows", None, "exec"),
    ("windows_cmd_lowercase", "run.cmd", ["a", "b"], "windows", None, "exec"),
    ("windows_cmd_uppercase", "Run.CMD", [], "windows", None, "exec"),
    ("windows_shell_amp", "dir & echo done", [], "windows", None, "shell"),
    ("windows_shell_args_quote", "echo", ["hello world"], "windows", None, "shell"),
    ("windows_shell_no_args", "dir", [], "windows", None, "shell"),
    ("windows_shell_empty_arg", "echo", [""], "windows", None, "shell"),
    ("windows_shell_quote_embedded", "echo", ["say \"hi\""], "windows", None, "shell"),
    ("windows_quoted_script_path", r'"C:\Path With Space\app.bat"', ["x"], "windows", None, "exec"),
    ("windows_exe_no_path_lookup", "C:\\Tools\\my.exe", ["--flag"], "windows", None, "exec"),
]


def main() -> None:
    out = []
    for name, cmd, args, plat, venv, mode in CASES:
        case = {
            "name": name,
            "command": cmd,
            "args": args,
            "platform": plat,
            "venv_path": venv,
            "run_mode": mode,
        }
        try:
            case["expected"] = build_command(cmd, args, platform=plat, venv_path=venv, run_mode=mode)
        except Exception as e:
            case["error"] = str(e)
        out.append(case)
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()

import os
import shlex
import subprocess
from typing import Annotated

import typer
from rich import box
from rich.console import Console
from rich.table import Table

from .client import HelmClient, HelmError
from .config import HELM_PIN, HELM_URL
from .stream import follow_logs

script_app = typer.Typer(help="Manage scripts", no_args_is_help=True)

out = Console()
err = Console(stderr=True)


def _die(msg: str) -> None:
    err.print(f"[red]{msg}[/red]")
    raise typer.Exit(1)


def _client() -> HelmClient:
    return HelmClient(HELM_URL, HELM_PIN)


def _build_local_cmd(script: dict) -> list[str]:
    command = (script.get("command") or "").strip()
    if not command:
        raise HelmError("Script command is empty")

    args = [str(a) for a in (script.get("args") or [])]
    run_mode = script.get("run_mode") or "exec"
    is_windows = os.name == "nt"

    if run_mode == "shell":
        line = command
        if args:
            if is_windows:
                line = f"{line} {subprocess.list2cmdline(args)}"
            else:
                line = f"{line} {' '.join(shlex.quote(a) for a in args)}"
        if is_windows:
            return ["cmd.exe", "/d", "/s", "/c", line]
        return ["bash", "-lc", line]

    parts = shlex.split(command, posix=not is_windows)
    if not parts:
        raise HelmError("Script command is empty")
    return [*parts, *args]


def _run_local(script: dict, prefix: list[str]) -> int:
    cmd = _build_local_cmd(script)
    full_cmd = [*prefix, *cmd]
    out.print(
        "[dim]Local run:[/dim] "
        + " ".join(shlex.quote(part) for part in full_cmd)
    )
    completed = subprocess.run(
        full_cmd,
        cwd=script.get("cwd") or None,
        check=False,
    )
    return int(completed.returncode)


@script_app.command("ls")
def script_ls():
    """List all scripts."""
    c = _client()
    try:
        scripts = c.scripts()
    except Exception as e:
        _die(str(e))
    finally:
        c.close()

    if not scripts:
        out.print("[dim]No scripts registered.[/dim]")
        return

    table = Table(box=box.SIMPLE, header_style="bold", show_edge=False)
    table.add_column("NAME", style="cyan", no_wrap=True)
    table.add_column("DESCRIPTION")
    table.add_column("CRON")
    table.add_column("ENABLED")

    for s in scripts:
        cron = s.get("cron_schedule") or "—"
        enabled = "[green]yes[/green]" if s.get("cron_enabled") else "no"
        table.add_row(s["name"], s.get("description") or "—", cron, enabled)

    out.print(table)


@script_app.command(
    "run",
    context_settings={"allow_extra_args": True, "ignore_unknown_options": True},
)
def script_run(
    ctx: typer.Context,
    name: Annotated[str, typer.Argument(help="Script name or ID")],
    follow: Annotated[bool, typer.Option("--follow", "-f", help="Stream logs after launch")] = False,
    local: Annotated[
        bool,
        typer.Option(
            "--local",
            "-l",
            help="Also run the script command locally in this terminal",
        ),
    ] = False,
):
    """Run a script in HELM. Add --local to run it locally too."""
    c = _client()
    try:
        s = c.resolve_script(name)
        result = c.script_run(s["id"])
        run_id = result.get("run_id")
        out.print(f"[green]Started[/green] [cyan]{s['name']}[/cyan]  run_id=[bold]{run_id}[/bold]")

        # Prefix can be passed after `--`, for example:
        # helmctl script run my-script --local -- wt new-tab
        prefix = list(ctx.args)
        if prefix:
            local = True

        if local:
            if follow:
                out.print("[yellow]--follow ignored in --local mode[/yellow]")
            code = _run_local(s, prefix)
            if code == 0:
                out.print("[green]Local run finished successfully[/green]")
            else:
                out.print(f"[red]Local run failed[/red] exit_code={code}")
                raise typer.Exit(code)
            return

        if follow:
            ws_url = c.ws_url(f"/ws/logs/script/{s['id']}")
            if HELM_PIN:
                ws_url += f"?pin={HELM_PIN}"
            c.close()
            follow_logs(ws_url, s["name"])
            return
    except HelmError as e:
        _die(str(e))
    except Exception as e:
        _die(str(e))
    finally:
        c.close()


@script_app.command("status")
def script_status(run_id: Annotated[int, typer.Argument(help="Run ID from 'script run'")]):
    """Show status of a script run."""
    c = _client()
    try:
        data = c.run_status(run_id)
    except HelmError as e:
        _die(str(e))
    finally:
        c.close()

    s = data.get("status", "unknown")
    color = {"running": "green", "completed": "green", "failed": "red", "error": "red"}.get(s, "white")
    out.print(f"[{color}]{s}[/{color}]")
    for k, v in data.items():
        if k != "status" and v is not None:
            out.print(f"[bold]{k}[/bold]: {v}")

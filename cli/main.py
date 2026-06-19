import json
from typing import Annotated, Optional

import typer
from rich import box
from rich.console import Console
from rich.table import Table
from rich.text import Text

from .client import HelmClient, HelmError
from .config import HELM_PIN, HELM_URL
from .scripts import script_app
from .stream import follow_logs

app = typer.Typer(name="helmctl", help="HELM service control CLI", no_args_is_help=True)
app.add_typer(script_app, name="script")

out = Console()
err = Console(stderr=True)


def _client() -> HelmClient:
    return HelmClient(HELM_URL, HELM_PIN)


def _die(msg: str) -> None:
    err.print(f"[red]{msg}[/red]")
    raise typer.Exit(1)


def _status_text(status: str) -> Text:
    colors = {
        "running": "green",
        "stopped": "dim",
        "starting": "yellow",
        "stopping": "yellow",
        "error": "bold red",
        "crashed": "bold red",
    }
    return Text(status, style=colors.get(status, "white"))


# ---------------------------------------------------------------------------
# Service commands
# ---------------------------------------------------------------------------


@app.command()
def ls():
    """List all services with their current status."""
    c = _client()
    try:
        services = c.services()
    except Exception as e:
        _die(str(e))
    finally:
        c.close()

    if not services:
        out.print("[dim]No services registered.[/dim]")
        return

    table = Table(box=box.SIMPLE, header_style="bold", show_edge=False)
    table.add_column("NAME", style="cyan", no_wrap=True)
    table.add_column("TYPE")
    table.add_column("STATUS")
    table.add_column("PID", justify="right")
    table.add_column("URL", style="dim")

    for svc in services:
        table.add_row(
            svc["name"],
            svc.get("type") or "—",
            _status_text(svc.get("status", "unknown")),
            str(svc["pid"]) if svc.get("pid") else "—",
            svc.get("url") or "—",
        )
    out.print(table)


@app.command()
def status(name: Annotated[str, typer.Argument(help="Service name or ID")]):
    """Show detailed info for a service."""
    c = _client()
    try:
        svc = c.resolve_service(name)
    except HelmError as e:
        _die(str(e))
    finally:
        c.close()

    for k, v in svc.items():
        if v not in (None, "", [], {}):
            out.print(f"[bold]{k}[/bold]: {v}")


def _service_action(name: str, action: str, verb: str) -> None:
    c = _client()
    try:
        svc = c.resolve_service(name)
        c.service_action(svc["id"], action)
        out.print(f"{verb} [cyan]{svc['name']}[/cyan]")
    except HelmError as e:
        _die(str(e))
    except Exception as e:
        _die(f"Server error: {e}")
    finally:
        c.close()


@app.command()
def start(name: Annotated[str, typer.Argument(help="Service name or ID")]):
    """Start a service."""
    _service_action(name, "start", "[green]Started[/green]")


@app.command()
def stop(name: Annotated[str, typer.Argument(help="Service name or ID")]):
    """Stop a service."""
    _service_action(name, "stop", "[yellow]Stopped[/yellow]")


@app.command()
def restart(name: Annotated[str, typer.Argument(help="Service name or ID")]):
    """Restart a service."""
    _service_action(name, "restart", "[cyan]Restarted[/cyan]")


@app.command()
def logs(
    name: Annotated[str, typer.Argument(help="Service name or ID")],
    follow: Annotated[bool, typer.Option("--follow", "-f")] = False,
    lines: Annotated[int, typer.Option("--lines", "-n")] = 100,
):
    """Show service logs. Use -f to stream live output."""
    c = _client()
    try:
        svc = c.resolve_service(name)
    except HelmError as e:
        c.close()
        _die(str(e))

    if follow:
        ws_url = c.ws_url(f"/ws/logs/service/{svc['id']}")
        if HELM_PIN:
            ws_url += f"?pin={HELM_PIN}"
        c.close()
        follow_logs(ws_url, svc["name"])
        return

    try:
        rows = c.service_logs(svc["id"], lines)
        for row in rows:
            text = row.get("line", row.get("text", ""))
            style = "dim" if row.get("stream") == "stderr" else ""
            out.print(text, style=style, highlight=False)
    except Exception as e:
        _die(str(e))
    finally:
        c.close()


# ---------------------------------------------------------------------------
# System commands
# ---------------------------------------------------------------------------


@app.command()
def info():
    """Show HELM system information."""
    c = _client()
    try:
        data = c.info()
    except Exception as e:
        _die(str(e))
    finally:
        c.close()
    for k, v in data.items():
        out.print(f"[bold]{k}[/bold]: {v}")


@app.command()
def health():
    """Check if HELM is reachable."""
    c = _client()
    try:
        data = c.health()
        ok = data.get("status") == "ok"
        out.print("[green]ok[/green]" if ok else f"[yellow]{data}[/yellow]")
    except Exception as e:
        err.print(f"[red]Unreachable:[/red] {e}")
        raise typer.Exit(1)
    finally:
        c.close()


@app.command("export")
def export_cmd(
    output: Annotated[Optional[str], typer.Argument(help="Output file (default: stdout)")] = None,
):
    """Export HELM config to JSON."""
    c = _client()
    try:
        data = c.export()
    except Exception as e:
        _die(str(e))
    finally:
        c.close()

    text = json.dumps(data, indent=2, ensure_ascii=False)
    if output:
        with open(output, "w", encoding="utf-8") as f:
            f.write(text)
        out.print(f"[green]Exported →[/green] {output}")
    else:
        print(text)


@app.command("import")
def import_cmd(file: Annotated[str, typer.Argument(help="JSON file to import")]):
    """Import HELM config from JSON file."""
    c = _client()
    try:
        c.import_config(file)
        out.print(f"[green]Imported[/green] {file}")
    except FileNotFoundError:
        _die(f"File not found: {file}")
    except Exception as e:
        _die(str(e))
    finally:
        c.close()


if __name__ == "__main__":
    app()

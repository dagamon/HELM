from typing import Annotated

import typer
from rich import box
from rich.console import Console
from rich.table import Table
from rich.text import Text

from .client import HelmClient, HelmError
from .config import HELM_PIN, HELM_URL

stack_app = typer.Typer(help="Manage stacks (groups of services)", no_args_is_help=True)

out = Console()
err = Console(stderr=True)


def _client() -> HelmClient:
    return HelmClient(HELM_URL, HELM_PIN)


def _die(msg: str) -> None:
    err.print(f"[red]{msg}[/red]")
    raise typer.Exit(1)


def _status_text(status: str) -> Text:
    colors = {"running": "green", "partial": "yellow", "stopped": "dim"}
    return Text(status, style=colors.get(status, "white"))


@stack_app.command()
def ls():
    """List all stacks."""
    c = _client()
    try:
        stacks = c.stacks()
    except Exception as e:
        _die(str(e))
    finally:
        c.close()

    if not stacks:
        out.print("[dim]No stacks defined.[/dim]")
        return

    table = Table(box=box.SIMPLE, header_style="bold", show_edge=False)
    table.add_column("NAME", style="cyan", no_wrap=True)
    table.add_column("STATUS")
    table.add_column("SERVICES", justify="right")
    table.add_column("RUNNING", justify="right")
    table.add_column("DESCRIPTION", style="dim")

    for st in stacks:
        table.add_row(
            st["name"],
            _status_text(st.get("status", "unknown")),
            str(st.get("service_count", 0)),
            str(st.get("running_count", 0)),
            st.get("description") or "—",
        )
    out.print(table)


@stack_app.command()
def status(name: Annotated[str, typer.Argument(help="Stack name or ID")]):
    """Show detailed info for a stack, including its services."""
    c = _client()
    try:
        st = c.resolve_stack(name)
        services = [s for s in c.services() if s.get("stack_id") == st["id"]]
    except HelmError as e:
        _die(str(e))
    finally:
        c.close()

    for k, v in st.items():
        if v not in (None, "", [], {}):
            out.print(f"[bold]{k}[/bold]: {v}")

    if services:
        out.print()
        table = Table(box=box.SIMPLE, header_style="bold", show_edge=False)
        table.add_column("NAME", style="cyan", no_wrap=True)
        table.add_column("STATUS")
        table.add_column("PID", justify="right")
        for svc in services:
            table.add_row(
                svc["name"],
                Text(
                    svc.get("status", "unknown"),
                    style={"running": "green", "stopped": "dim"}.get(
                        svc.get("status", ""), "bold red"
                    ),
                ),
                str(svc["pid"]) if svc.get("pid") else "—",
            )
        out.print(table)


def _report(result: dict) -> None:
    for member in result.get("services", []):
        outcome = member.get("outcome", "?")
        style = {
            "started": "green",
            "stopped": "yellow",
            "already_running": "dim",
            "already_stopped": "dim",
            "failed": "bold red",
        }.get(outcome, "white")
        line = f"  [{style}]{outcome}[/{style}] {member['name']}"
        if member.get("error"):
            line += f" [red]({member['error']})[/red]"
        out.print(line)


def _stack_action(name: str, action: str, verb: str) -> None:
    c = _client()
    try:
        st = c.resolve_stack(name)
        result = c.stack_action(st["id"], action)
        out.print(f"{verb} [cyan]{st['name']}[/cyan]")
        _report(result)
        if result.get("status") == "partial":
            raise typer.Exit(1)
    except HelmError as e:
        _die(str(e))
    except typer.Exit:
        raise
    except Exception as e:
        _die(f"Server error: {e}")
    finally:
        c.close()


@stack_app.command()
def start(name: Annotated[str, typer.Argument(help="Stack name or ID")]):
    """Start all services in a stack (dependencies first)."""
    _stack_action(name, "start", "[green]Starting[/green]")


@stack_app.command()
def stop(name: Annotated[str, typer.Argument(help="Stack name or ID")]):
    """Stop all services in a stack (dependents first)."""
    _stack_action(name, "stop", "[yellow]Stopping[/yellow]")


@stack_app.command()
def restart(name: Annotated[str, typer.Argument(help="Stack name or ID")]):
    """Restart all services in a stack."""
    _stack_action(name, "restart", "[cyan]Restarting[/cyan]")

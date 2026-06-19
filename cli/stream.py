"""WebSocket log streaming for --follow mode."""
import asyncio
import json
import sys

from rich.console import Console

err = Console(stderr=True)


def follow_logs(ws_url: str, name: str) -> None:
    """Stream log lines from a WebSocket endpoint until Ctrl+C."""
    import websockets

    async def _run() -> None:
        Console().print(f"[dim]Streaming [bold]{name}[/bold] — Ctrl+C to stop[/dim]")
        try:
            async with websockets.connect(ws_url) as ws:
                async for message in ws:
                    try:
                        data = json.loads(message)
                        text = data.get("text", data.get("line", str(data)))
                    except (json.JSONDecodeError, AttributeError):
                        text = message
                    print(text, end="" if text.endswith("\n") else "\n", flush=True)
        except KeyboardInterrupt:
            pass
        except Exception as e:
            err.print(f"[red]Stream error:[/red] {e}")
            sys.exit(1)

    asyncio.run(_run())

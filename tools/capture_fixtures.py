"""Capture snapshot fixtures from running Python HELM for Rust rewrite contract testing."""

import asyncio
import json
import sys
from pathlib import Path

import httpx
import websockets

BASE = "http://localhost:7010"
WS_BASE = "ws://localhost:7010"
OUT_REST = Path(__file__).parent.parent / "tests" / "fixtures" / "rest"
OUT_WS = Path(__file__).parent.parent / "tests" / "fixtures" / "ws"
OUT_REST.mkdir(parents=True, exist_ok=True)
OUT_WS.mkdir(parents=True, exist_ok=True)


def save(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"  saved {path.name}")


async def capture_rest(client: httpx.AsyncClient) -> dict:
    """Returns {service_id, script_id} for further endpoint captures."""
    ids = {}

    # --- health ---
    r = await client.get(f"{BASE}/api/health")
    save(OUT_REST / "health.json", {
        "method": "GET", "path": "/api/health",
        "status": r.status_code, "response": r.json()
    })

    # --- system/info ---
    r = await client.get(f"{BASE}/api/system/info")
    body = r.json()
    # normalise volatile fields
    body["uptime_seconds"] = "<NUM>"
    save(OUT_REST / "system_info.json", {
        "method": "GET", "path": "/api/system/info",
        "status": r.status_code, "response": body
    })

    # --- services list ---
    r = await client.get(f"{BASE}/api/services")
    services = r.json()
    # normalise pids/status
    for s in services:
        s["status"] = "<STATUS>"
        s["pid"] = "<PID>"
    save(OUT_REST / "services_list.json", {
        "method": "GET", "path": "/api/services",
        "status": r.status_code, "response": services
    })
    if services:
        ids["service_id"] = services[0]["id"]

    # --- service detail ---
    if "service_id" in ids:
        sid = ids["service_id"]
        r = await client.get(f"{BASE}/api/services/{sid}")
        body = r.json()
        body["status"] = "<STATUS>"
        body["pid"] = "<PID>"
        save(OUT_REST / "service_get.json", {
            "method": "GET", "path": f"/api/services/{sid}",
            "status": r.status_code, "response": body
        })

    # --- service 404 ---
    r = await client.get(f"{BASE}/api/services/99999")
    save(OUT_REST / "service_404.json", {
        "method": "GET", "path": "/api/services/99999",
        "status": r.status_code, "response": r.json()
    })

    # --- service logs ---
    if "service_id" in ids:
        sid = ids["service_id"]
        r = await client.get(f"{BASE}/api/services/{sid}/logs?limit=10")
        save(OUT_REST / "service_logs.json", {
            "method": "GET", "path": f"/api/services/{sid}/logs",
            "status": r.status_code, "response": r.json()
        })

    # --- service metrics ---
    if "service_id" in ids:
        sid = ids["service_id"]
        r = await client.get(f"{BASE}/api/services/{sid}/metrics")
        save(OUT_REST / "service_metrics.json", {
            "method": "GET", "path": f"/api/services/{sid}/metrics",
            "status": r.status_code, "response": r.json()
        })

    # --- service create + update + delete (round-trip) ---
    r = await client.post(f"{BASE}/api/services", json={
        "name": "__fixture_test__",
        "type": "shell",
        "description": "fixture capture test service",
        "command": "echo hello",
        "auto_start": False,
        "restart_on_crash": False,
        "platform": "all",
    })
    created = r.json()
    created["status"] = "<STATUS>"
    created["pid"] = "<PID>"
    save(OUT_REST / "service_create.json", {
        "method": "POST", "path": "/api/services",
        "request": {"name": "__fixture_test__", "type": "shell", "command": "echo hello"},
        "status": r.status_code, "response": created
    })
    new_id = created["id"]

    r = await client.put(f"{BASE}/api/services/{new_id}", json={"description": "updated"})
    body = r.json()
    body["status"] = "<STATUS>"
    body["pid"] = "<PID>"
    save(OUT_REST / "service_update.json", {
        "method": "PUT", "path": f"/api/services/{new_id}",
        "request": {"description": "updated"},
        "status": r.status_code, "response": body
    })

    r = await client.delete(f"{BASE}/api/services/{new_id}")
    save(OUT_REST / "service_delete.json", {
        "method": "DELETE", "path": f"/api/services/{new_id}",
        "status": r.status_code, "response": None
    })

    # --- scripts list ---
    r = await client.get(f"{BASE}/api/scripts")
    scripts = r.json()
    save(OUT_REST / "scripts_list.json", {
        "method": "GET", "path": "/api/scripts",
        "status": r.status_code, "response": scripts
    })
    if scripts:
        ids["script_id"] = scripts[0]["id"]

    # --- script 404 ---
    r = await client.get(f"{BASE}/api/scripts/99999")
    try:
        body = r.json()
    except Exception:
        body = {"detail": r.text}
    save(OUT_REST / "script_404.json", {
        "method": "GET", "path": "/api/scripts/99999",
        "status": r.status_code, "response": body
    })

    # --- script create + delete ---
    r = await client.post(f"{BASE}/api/scripts", json={
        "name": "__fixture_script__",
        "command": "echo hi",
        "run_mode": "exec",
        "platform": "all",
    })
    created_s = r.json()
    save(OUT_REST / "script_create.json", {
        "method": "POST", "path": "/api/scripts",
        "request": {"name": "__fixture_script__", "command": "echo hi"},
        "status": r.status_code, "response": created_s
    })
    new_sid = created_s["id"]

    r = await client.delete(f"{BASE}/api/scripts/{new_sid}")
    save(OUT_REST / "script_delete.json", {
        "method": "DELETE", "path": f"/api/scripts/{new_sid}",
        "status": r.status_code, "response": None
    })

    # --- export ---
    r = await client.get(f"{BASE}/api/export")
    body = r.json()
    # strip volatile created_at values for stable snapshot
    for svc in body.get("services", []):
        svc.pop("created_at", None)
        svc.pop("updated_at", None)
    for scr in body.get("scripts", []):
        scr.pop("created_at", None)
    save(OUT_REST / "export.json", {
        "method": "GET", "path": "/api/export",
        "status": r.status_code, "response": body
    })

    # --- faq ---
    r = await client.get(f"{BASE}/api/faq")
    save(OUT_REST / "faq_list.json", {
        "method": "GET", "path": "/api/faq",
        "status": r.status_code, "response": r.json()
    })

    return ids


async def capture_ws(ids: dict) -> None:
    # --- /ws/status ---
    msgs = []
    try:
        async with websockets.connect(f"{WS_BASE}/ws/status", open_timeout=5) as ws:
            for _ in range(5):
                try:
                    raw = await asyncio.wait_for(ws.recv(), timeout=3.0)
                    msg = json.loads(raw)
                    msg["pid"] = "<PID>"
                    msg["ts"] = "<ISO>"
                    msgs.append(msg)
                except asyncio.TimeoutError:
                    break
    except Exception as e:
        print(f"  ws/status capture skipped: {e}")

    save(OUT_WS / "status_sample.json", {
        "endpoint": "/ws/status",
        "sample_messages": msgs,
        "payload_shape": {
            "entity_type": "service|script",
            "entity_id": "<INT>",
            "status": "running|stopped|crashed|success",
            "pid": "<PID>|null",
            "metrics": {"cpu_percent": "<FLOAT>", "memory_mb": "<FLOAT>"}
        }
    })
    print(f"  ws/status: captured {len(msgs)} messages")

    # --- /ws/logs ---
    if "service_id" in ids:
        sid = ids["service_id"]
        log_msgs = []
        try:
            async with websockets.connect(
                f"{WS_BASE}/ws/logs/service/{sid}", open_timeout=5
            ) as ws:
                for _ in range(10):
                    try:
                        raw = await asyncio.wait_for(ws.recv(), timeout=2.0)
                        msg = json.loads(raw)
                        msg["ts"] = "<ISO>"
                        log_msgs.append(msg)
                    except asyncio.TimeoutError:
                        break
        except Exception as e:
            print(f"  ws/logs capture skipped: {e}")

        save(OUT_WS / "logs_sample.json", {
            "endpoint": f"/ws/logs/service/{sid}",
            "sample_messages": log_msgs,
            "payload_shape": {
                "stream": "stdout|stderr",
                "text": "<STRING>",
                "ts": "<ISO>"
            }
        })
        print(f"  ws/logs: captured {len(log_msgs)} messages")


async def capture_schema() -> None:
    import subprocess
    db = Path(__file__).parent.parent / "data" / "dashboard.db"
    out = Path(__file__).parent.parent / "tests" / "fixtures" / "schema.sql"
    result = subprocess.run(
        ["sqlite3", str(db), ".schema"],
        capture_output=True, text=True
    )
    if result.returncode == 0:
        out.write_text(result.stdout, encoding="utf-8")
        print(f"  schema dump saved ({len(result.stdout)} chars)")
    else:
        # fallback: read schema via Python sqlite3
        import sqlite3 as sq
        con = sq.connect(str(db))
        rows = con.execute(
            "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type, name"
        ).fetchall()
        con.close()
        schema = "\n\n".join(r[0] for r in rows)
        out.write_text(schema, encoding="utf-8")
        print(f"  schema dump saved via Python fallback ({len(schema)} chars)")


async def main() -> None:
    print("=== S0: Capturing fixtures from Python HELM ===")
    async with httpx.AsyncClient(timeout=10) as client:
        print("[REST]")
        ids = await capture_rest(client)
        print(f"[WS] service_id={ids.get('service_id')}")
        await capture_ws(ids)
    print("[SCHEMA]")
    await capture_schema()
    print(f"\nDone. REST fixtures: {len(list(OUT_REST.glob('*.json')))}, WS fixtures: {len(list(OUT_WS.glob('*.json')))}")


if __name__ == "__main__":
    asyncio.run(main())

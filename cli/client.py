import httpx
from typing import Any


class HelmError(Exception):
    pass


class HelmClient:
    def __init__(self, base_url: str, pin: str = "") -> None:
        self.base_url = base_url.rstrip("/")
        headers = {"x-dashboard-pin": pin} if pin else {}
        self._http = httpx.Client(base_url=self.base_url, headers=headers, timeout=15.0)

    def close(self) -> None:
        self._http.close()

    def _get(self, path: str) -> Any:
        r = self._http.get(path)
        if r.status_code == 404:
            raise HelmError(f"Not found: {path}")
        r.raise_for_status()
        return r.json()

    def _post(self, path: str, **kwargs: Any) -> Any:
        r = self._http.post(path, **kwargs)
        r.raise_for_status()
        return r.json()

    # Services
    def services(self) -> list[dict]:
        return self._get("/api/services")

    def resolve_service(self, name_or_id: str) -> dict:
        for svc in self.services():
            if svc["name"] == name_or_id or str(svc["id"]) == name_or_id:
                return svc
        raise HelmError(f"Service '{name_or_id}' not found")

    def service_action(self, service_id: int, action: str) -> dict:
        return self._post(f"/api/services/{service_id}/{action}")

    def service_logs(self, service_id: int, limit: int = 100) -> list[dict]:
        return self._get(f"/api/services/{service_id}/logs?limit={limit}")

    # Stacks
    def stacks(self) -> list[dict]:
        return self._get("/api/stacks")

    def resolve_stack(self, name_or_id: str) -> dict:
        for st in self.stacks():
            if st["name"] == name_or_id or str(st["id"]) == name_or_id:
                return st
        raise HelmError(f"Stack '{name_or_id}' not found")

    def stack_action(self, stack_id: int, action: str) -> dict:
        return self._post(f"/api/stacks/{stack_id}/{action}")

    # Scripts
    def scripts(self) -> list[dict]:
        return self._get("/api/scripts")

    def resolve_script(self, name_or_id: str) -> dict:
        for s in self.scripts():
            if s["name"] == name_or_id or str(s["id"]) == name_or_id:
                return s
        raise HelmError(f"Script '{name_or_id}' not found")

    def script_run(self, script_id: int) -> dict:
        return self._post(f"/api/scripts/{script_id}/run")

    def run_status(self, run_id: int) -> dict:
        return self._get(f"/api/scripts/runs/{run_id}")

    # System
    def health(self) -> dict:
        return self._get("/api/health")

    def info(self) -> dict:
        return self._get("/api/system/info")

    def export(self) -> dict:
        return self._get("/api/export")

    def import_config(self, filepath: str) -> dict:
        with open(filepath, "rb") as f:
            return self._post("/api/import", files={"file": f})

    def ws_url(self, path: str) -> str:
        return self.base_url.replace("http://", "ws://").replace("https://", "wss://") + path

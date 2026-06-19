import { useEffect, useRef } from "react";
import type { StatusEvent } from "@/api/types";
import { useServices } from "@/store/services";

export function useStatusStream() {
  const wsRef = useRef<WebSocket | null>(null);
  const patchLocal = useServices((s) => s.patchLocal);

  useEffect(() => {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/ws/status`;

    function connect() {
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onmessage = (e) => {
        const event: StatusEvent = JSON.parse(e.data);
        if (event.entity_type === "service") {
          const patch: Partial<{ status: string; pid: number | null; metrics: typeof event.metrics }> = {
            status: event.status,
            pid: event.pid,
          };
          if (event.metrics) {
            patch.metrics = event.metrics;
          }
          patchLocal(event.entity_id, patch);
        }
      };

      ws.onclose = () => {
        setTimeout(connect, 3000);
      };
    }

    connect();
    return () => {
      wsRef.current?.close();
    };
  }, [patchLocal]);
}

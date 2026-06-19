import { useEffect, useRef, useCallback } from "react";
import type { LogLine } from "@/api/types";

interface UseLogStreamOptions {
  entityType: string;
  entityId: number;
  onLine: (line: LogLine) => void;
}

export function useLogStream({ entityType, entityId, onLine }: UseLogStreamOptions) {
  const wsRef = useRef<WebSocket | null>(null);
  const onLineRef = useRef(onLine);
  onLineRef.current = onLine;

  const connect = useCallback(() => {
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = `${proto}//${location.host}/ws/logs/${entityType}/${entityId}`;
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onmessage = (e) => {
      onLineRef.current(JSON.parse(e.data));
    };

    ws.onclose = () => {
      setTimeout(() => connect(), 3000);
    };
  }, [entityType, entityId]);

  useEffect(() => {
    connect();
    return () => {
      wsRef.current?.close();
    };
  }, [connect]);
}

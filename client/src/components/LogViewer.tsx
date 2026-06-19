import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import { useLogStream } from "@/hooks/useLogStream";
import type { LogLine } from "@/api/types";

interface LogViewerProps {
  entityType: string;
  entityId: number;
}

export function LogViewer({ entityType, entityId }: LogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      theme: {
        background: "#000000",
        foreground: "#ececef",
        cursor: "#ececef",
        selectionBackground: "#3b82f620",
      },
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      cursorBlink: false,
      disableStdin: true,
      scrollback: 5000,
      convertEol: true,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());

    term.open(containerRef.current);
    fit.fit();

    termRef.current = term;
    fitRef.current = fit;

    const observer = new ResizeObserver(() => fit.fit());
    observer.observe(containerRef.current);

    return () => {
      observer.disconnect();
      term.dispose();
      termRef.current = null;
    };
  }, []);

  const handleLine = (line: LogLine) => {
    const term = termRef.current;
    if (!term) return;
    const text = line.text ?? line.line ?? "";
    if (line.stream === "stderr") {
      term.writeln(`\x1b[31m${text}\x1b[0m`);
    } else {
      term.writeln(text);
    }
  };

  useLogStream({ entityType, entityId, onLine: handleLine });

  return (
    <div
      ref={containerRef}
      className="xterm-container w-full h-full min-h-[300px] rounded-lg overflow-hidden bg-black"
    />
  );
}

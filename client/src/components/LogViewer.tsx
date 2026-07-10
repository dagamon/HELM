import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Check, Copy } from "lucide-react";
import "@xterm/xterm/css/xterm.css";
import { useLogStream } from "@/hooks/useLogStream";
import { usePrefs } from "@/store/prefs";
import type { LogLine } from "@/api/types";

interface LogViewerProps {
  entityType: string;
  entityId: number;
}

export function LogViewer({ entityType, entityId }: LogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const [copied, setCopied] = useState(false);
  const fontSize = usePrefs((s) => s.terminalFontSize);
  const autoScroll = usePrefs((s) => s.logAutoScroll);
  const autoScrollRef = useRef(autoScroll);
  autoScrollRef.current = autoScroll;

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      theme: {
        background: "#000000",
        foreground: "#ececef",
        cursor: "#ececef",
        selectionBackground: "#3b82f620",
      },
      fontSize,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      cursorBlink: false,
      disableStdin: true,
      scrollback: 5000,
      convertEol: true,
    });

    const fit = new FitAddon();
    term.loadAddon(fit);
    term.loadAddon(new WebLinksAddon());

    // Ctrl+C copies the active selection (Ctrl+Shift+C too). With nothing
    // selected, let the key fall through so it never blocks the browser.
    term.attachCustomKeyEventHandler((e) => {
      const copyCombo =
        e.type === "keydown" &&
        (e.ctrlKey || e.metaKey) &&
        (e.key === "c" || e.key === "C");
      if (copyCombo && term.hasSelection()) {
        const sel = term.getSelection();
        if (sel) {
          navigator.clipboard?.writeText(sel).catch(() => {});
          e.preventDefault();
          return false;
        }
      }
      return true;
    });

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
    // Terminal is created once; font size is re-applied by a separate effect.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Re-apply font size live without tearing down the terminal/scrollback.
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    term.options.fontSize = fontSize;
    fitRef.current?.fit();
  }, [fontSize]);

  const handleLine = (line: LogLine) => {
    const term = termRef.current;
    if (!term) return;
    const text = line.text ?? line.line ?? "";
    if (line.stream === "stderr") {
      term.writeln(`\x1b[31m${text}\x1b[0m`);
    } else {
      term.writeln(text);
    }
    if (autoScrollRef.current) term.scrollToBottom();
  };

  useLogStream({ entityType, entityId, onLine: handleLine });

  const handleCopy = async () => {
    const term = termRef.current;
    if (!term) return;
    const buffer = term.buffer.active;
    const lines: string[] = [];
    for (let i = 0; i < buffer.length; i++) {
      const line = buffer.getLine(i);
      if (line) lines.push(line.translateToString(true));
    }
    const text = lines.join("\n").replace(/\n+$/, "");
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard unavailable (e.g. insecure context); ignore silently
    }
  };

  return (
    <div className="relative w-full h-full min-h-[300px]">
      <button
        type="button"
        onClick={handleCopy}
        title="Copy logs"
        className="absolute top-2 right-2 z-10 flex items-center gap-1.5 px-2 py-1 rounded-md bg-surface/80 backdrop-blur-sm border border-border text-xs text-text-muted hover:text-text hover:bg-surface-hover transition-colors"
      >
        {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
        {copied ? "Copied" : "Copy"}
      </button>
      <div
        ref={containerRef}
        className="xterm-container w-full h-full min-h-[300px] rounded-lg overflow-hidden bg-black"
      />
    </div>
  );
}

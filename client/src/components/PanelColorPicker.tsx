import { useState } from "react";
import { Palette, Ban } from "lucide-react";
import { useThemeStore } from "@/store/theme";

/**
 * Swatch popover for picking a card's panel color. Only colors from the
 * active theme's `panels` palette are offered; the stored value is the
 * palette key, so switching themes remaps every card automatically.
 */
export function PanelColorPicker({
  value,
  onChange,
}: {
  value: string | null;
  onChange: (key: string) => void;
}) {
  const panels = useThemeStore((s) => s.panels);
  const [open, setOpen] = useState(false);
  const entries = Object.entries(panels ?? {});
  if (entries.length === 0) return null;

  const pick = (key: string) => {
    setOpen(false);
    onChange(key);
  };

  return (
    <div className="relative" onClick={(e) => e.stopPropagation()}>
      <button
        title="Panel color"
        onClick={() => setOpen((v) => !v)}
        className="p-1.5 rounded-md hover:bg-surface-raised text-text-tertiary hover:text-text-muted transition-colors"
      >
        <Palette className="w-3.5 h-3.5" />
      </button>
      {open && (
        <div
          className="absolute bottom-full left-0 mb-1.5 z-20 flex items-center gap-1.5 rounded-lg border border-border bg-surface-raised p-2 shadow-lg"
          onMouseLeave={() => setOpen(false)}
        >
          <button
            title="Default"
            onClick={() => pick("")}
            className={`flex items-center justify-center w-5 h-5 rounded-full border transition-shadow ${
              !value ? "ring-2 ring-accent border-transparent" : "border-border"
            }`}
          >
            <Ban className="w-3 h-3 text-text-tertiary" />
          </button>
          {entries.map(([key, hex]) => (
            <button
              key={key}
              title={key}
              onClick={() => pick(key)}
              className={`w-5 h-5 rounded-full border transition-shadow ${
                value === key
                  ? "ring-2 ring-accent border-transparent"
                  : "border-black/20"
              }`}
              style={{ background: hex }}
            />
          ))}
        </div>
      )}
    </div>
  );
}

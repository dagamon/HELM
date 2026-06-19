import { useEffect, useRef, useState } from "react";
import { Tag, ChevronDown } from "lucide-react";

interface TagsDropdownProps {
  tags: string[];
  selected: string[];
  onToggle: (tag: string) => void;
}

export function TagsDropdown({ tags, selected, onToggle }: TagsDropdownProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className={`flex items-center gap-1.5 px-2.5 py-1.5 text-xs rounded-lg border transition-colors ${
          selected.length > 0
            ? "border-border-hover text-text bg-surface-hover"
            : "border-border text-text-muted hover:text-text hover:border-border-hover"
        }`}
      >
        <Tag className="w-3 h-3" />
        Tags
        {selected.length > 0 && (
          <span className="text-accent ml-0.5">{selected.length}</span>
        )}
        <ChevronDown
          className={`w-3 h-3 transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>

      {open && (
        <div className="absolute top-full mt-1 left-0 w-48 bg-surface border border-border rounded-lg shadow-lg z-30 py-1 max-h-48 overflow-y-auto">
          {tags.map((tag) => (
            <button
              key={tag}
              onClick={() => onToggle(tag)}
              className="flex items-center gap-2 w-full px-3 py-1.5 text-xs text-left hover:bg-surface-hover transition-colors"
            >
              <span
                className={`w-3.5 h-3.5 rounded border flex items-center justify-center shrink-0 ${
                  selected.includes(tag)
                    ? "bg-accent border-accent text-white"
                    : "border-border-hover"
                }`}
              >
                {selected.includes(tag) && (
                  <svg
                    className="w-2.5 h-2.5"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                    strokeWidth={3}
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                )}
              </span>
              <span className={selected.includes(tag) ? "text-text" : "text-text-muted"}>
                {tag}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

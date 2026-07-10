import { useEffect, useState, type ReactNode } from "react";
import { X, type LucideIcon } from "lucide-react";

export interface SplitSection {
  id: string;
  label: string;
  icon: LucideIcon;
  hint?: string;
  /** Show a danger dot in the rail (e.g. unmet required fields live here). */
  invalid?: boolean;
  content: ReactNode;
}

interface SplitModalProps {
  title: string;
  subtitle?: string;
  sections: SplitSection[];
  footer: ReactNode;
  onClose: () => void;
}

/**
 * Two-pane editor modal: a section rail on the left, the active section's
 * fields on the right, actions pinned to the footer. Fixed 4:3 stage so long
 * forms stop being one endless scroll. Collapses the rail to a top tab strip
 * on narrow viewports.
 */
export function SplitModal({ title, subtitle, sections, footer, onClose }: SplitModalProps) {
  const [active, setActive] = useState(sections[0]?.id);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const current = sections.find((s) => s.id === active) ?? sections[0];

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/55 backdrop-blur-sm p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="bg-surface/80 backdrop-blur-xl border border-border/70 rounded-2xl shadow-2xl flex flex-col w-[min(900px,96vw)] h-[min(680px,92vh)] overflow-hidden">
        {/* Header */}
        <div className="flex items-start justify-between px-8 pt-6 pb-5 border-b border-border/60 shrink-0">
          <div>
            <h2 className="text-lg font-semibold leading-tight tracking-tight">{title}</h2>
            {subtitle && <p className="text-xs text-text-muted mt-1">{subtitle}</p>}
          </div>
          <button
            onClick={onClose}
            aria-label="Close"
            className="p-1.5 -mr-1.5 rounded-md hover:bg-surface-hover text-text-muted transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body: rail + content */}
        <div className="flex flex-1 min-h-0 flex-col sm:flex-row">
          {/* Rail */}
          <nav className="shrink-0 sm:w-56 border-b sm:border-b-0 sm:border-r border-border/60 bg-bg/30 p-3 flex sm:flex-col gap-1 overflow-x-auto sm:overflow-y-auto">
            {sections.map((s, i) => {
              const isActive = s.id === current?.id;
              const Icon = s.icon;
              return (
                <button
                  key={s.id}
                  type="button"
                  onClick={() => setActive(s.id)}
                  className={`group relative flex items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm text-left whitespace-nowrap transition-colors ${
                    isActive
                      ? "bg-surface-raised text-text"
                      : "text-text-muted hover:text-text hover:bg-surface-hover"
                  }`}
                >
                  <span
                    className={`absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-full bg-accent transition-opacity ${
                      isActive ? "opacity-100" : "opacity-0"
                    }`}
                  />
                  <Icon className={`w-4 h-4 shrink-0 ${isActive ? "text-accent" : ""}`} />
                  <span className="flex-1">{s.label}</span>
                  {s.invalid && (
                    <span className="w-1.5 h-1.5 rounded-full bg-danger shrink-0" title="Needs attention" />
                  )}
                  <span className="hidden sm:block text-[10px] font-mono text-text-tertiary tabular-nums">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                </button>
              );
            })}
          </nav>

          {/* Content */}
          <div className="flex-1 min-h-0 overflow-y-auto px-8 py-7">
            {current?.hint && (
              <p className="text-xs text-text-muted mb-5">{current.hint}</p>
            )}
            {current?.content}
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-8 py-5 border-t border-border/60 bg-bg/30 shrink-0">
          {footer}
        </div>
      </div>
    </div>
  );
}

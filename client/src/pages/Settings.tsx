import { useEffect, useState } from "react";
import {
  Check,
  RotateCcw,
  Palette,
  SlidersHorizontal,
  ArrowUpCircle,
  Loader2,
  FolderCode,
} from "lucide-react";
import type { Theme } from "@/api/types";
import { useThemeStore } from "@/store/theme";
import { usePrefs } from "@/store/prefs";
import { api } from "@/api/client";
import type { UpdateStatus } from "@/api/types";

function ThemeCard({ theme }: { theme: Theme }) {
  const active = useThemeStore((s) => s.active === theme.name);
  const setTheme = useThemeStore((s) => s.setTheme);
  const strip = ["bg", "surface", "accent", "success", "danger"];
  const panels = Object.entries(theme.panels ?? {});

  return (
    <button
      type="button"
      onClick={() => setTheme(theme.name)}
      title={theme.hint}
      className={`text-left rounded-xl border p-3.5 transition-colors ${
        active
          ? "border-accent bg-surface-hover"
          : "border-border hover:border-border-hover hover:bg-surface-hover"
      }`}
    >
      <div className="flex items-center gap-1 mb-2.5">
        {strip.map((k) => (
          <span
            key={k}
            className="h-5 flex-1 rounded-sm border border-black/20"
            style={{ background: theme.colors[k] }}
          />
        ))}
      </div>
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium truncate">{theme.label}</span>
        {active && <Check className="w-4 h-4 text-accent shrink-0" />}
      </div>
      {theme.hint && (
        <div className="text-xs text-text-muted mt-0.5 truncate">{theme.hint}</div>
      )}
      {panels.length > 0 && (
        <div className="flex items-center gap-1.5 mt-2.5">
          <span className="text-[10px] uppercase tracking-wide text-text-tertiary mr-0.5">
            panels
          </span>
          {panels.map(([key, hex]) => (
            <span
              key={key}
              title={key}
              className="w-3.5 h-3.5 rounded-full border border-black/20"
              style={{ background: hex }}
            />
          ))}
        </div>
      )}
    </button>
  );
}

function Appearance() {
  const themes = useThemeStore((s) => s.themes);
  const loadThemes = useThemeStore((s) => s.loadThemes);

  useEffect(() => {
    loadThemes();
  }, [loadThemes]);

  return (
    <div className="max-w-3xl space-y-8">
      <div>
        <h1 className="text-lg font-semibold">Themes</h1>
        <p className="text-text-muted text-sm mt-1">
          Pick a theme for the whole dashboard. Each theme also defines the
          panel colors available for service and stack cards.
        </p>
      </div>

      {themes.length === 0 ? (
        <div className="text-sm text-text-muted">No themes found.</div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {themes.map((t) => (
            <ThemeCard key={t.name} theme={t} />
          ))}
        </div>
      )}

      <section className="rounded-lg border border-border bg-surface p-4 flex items-start gap-3">
        <FolderCode className="w-4 h-4 text-text-muted shrink-0 mt-0.5" />
        <div className="text-xs text-text-muted leading-relaxed">
          <span className="text-text">Custom themes</span> are JSON files in the{" "}
          <span className="font-mono">themes/</span> folder of the HELM repo.
          Copy an existing file, change <span className="font-mono">name</span>,{" "}
          <span className="font-mono">label</span>,{" "}
          <span className="font-mono">colors</span> and the{" "}
          <span className="font-mono">panels</span> palette — it appears here
          after a page reload. There is no in-app editor.
        </div>
      </section>
    </div>
  );
}

function Toggle({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-4 py-2 cursor-pointer">
      <span>
        <span className="text-sm">{label}</span>
        {hint && <span className="block text-xs text-text-muted mt-0.5">{hint}</span>}
      </span>
      <button
        type="button"
        onClick={() => onChange(!checked)}
        className={`relative w-10 h-5.5 shrink-0 rounded-full transition-colors ${
          checked ? "bg-accent" : "bg-surface-hover"
        }`}
        style={{ height: "1.375rem" }}
        role="switch"
        aria-checked={checked}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-4.5 h-4.5 rounded-full bg-white transition-transform ${
            checked ? "translate-x-[1.125rem]" : ""
          }`}
          style={{ width: "1.125rem", height: "1.125rem" }}
        />
      </button>
    </label>
  );
}

function GeneralSettings() {
  const p = usePrefs();

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-lg font-semibold">General</h1>
        <p className="text-text-muted text-sm mt-1">
          Local UX preferences. Stored in this browser.
        </p>
      </div>

      <section className="space-y-3">
        <h2 className="text-xs uppercase tracking-wide text-text-muted">Terminal</h2>
        <div className="rounded-lg border border-border bg-surface p-4 space-y-3">
          <div>
            <div className="flex items-center justify-between">
              <span className="text-sm">Log font size</span>
              <span className="text-xs text-text-muted tabular-nums">{p.terminalFontSize}px</span>
            </div>
            <input
              type="range"
              min={9}
              max={22}
              value={p.terminalFontSize}
              onChange={(e) => p.set("terminalFontSize", Number(e.target.value))}
              className="w-full mt-2 accent-[var(--color-accent)]"
            />
          </div>
          <div className="border-t border-border/60 pt-1">
            <Toggle
              label="Auto-scroll logs"
              hint="Keep the viewport pinned to the newest line."
              checked={p.logAutoScroll}
              onChange={(v) => p.set("logAutoScroll", v)}
            />
          </div>
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="text-xs uppercase tracking-wide text-text-muted">Safety</h2>
        <div className="rounded-lg border border-border bg-surface p-4">
          <Toggle
            label="Confirm destructive actions"
            hint="Ask before deleting services, scripts, or applying updates."
            checked={p.confirmDestructive}
            onChange={(v) => p.set("confirmDestructive", v)}
          />
        </div>
      </section>

      <button
        type="button"
        onClick={p.reset}
        className="inline-flex items-center gap-1.5 text-xs text-text-muted hover:text-text transition-colors"
      >
        <RotateCcw className="w-3.5 h-3.5" />
        Reset preferences
      </button>
    </div>
  );
}

function UpdatesSettings() {
  const p = usePrefs();
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [applying, setApplying] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);

  const check = async () => {
    setChecking(true);
    setMsg(null);
    try {
      setStatus(await api.checkUpdate());
    } catch (e) {
      setMsg(`Check failed: ${e instanceof Error ? e.message : e}`);
    } finally {
      setChecking(false);
    }
  };

  useEffect(() => {
    check();
  }, []);

  const apply = async () => {
    if (
      p.confirmDestructive &&
      !window.confirm("Pull latest, rebuild and restart HELM now? The dashboard will be briefly unavailable.")
    ) {
      return;
    }
    setApplying(true);
    setMsg(null);
    try {
      const r = await api.applyUpdate();
      setMsg(`Update started. Log: ${r.log}. HELM will restart shortly.`);
    } catch (e) {
      setMsg(`Update failed to start: ${e instanceof Error ? e.message : e}`);
      setApplying(false);
    }
  };

  return (
    <div className="max-w-2xl space-y-8">
      <div>
        <h1 className="text-lg font-semibold">Updates</h1>
        <p className="text-text-muted text-sm mt-1">
          Pull and rebuild HELM from its git checkout.
        </p>
      </div>

      <section className="rounded-lg border border-border bg-surface p-4 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm font-medium">
              {status
                ? status.update_available
                  ? `${status.behind} commit${status.behind === 1 ? "" : "s"} behind`
                  : "Up to date"
                : "—"}
            </div>
            <div className="text-xs text-text-muted mt-0.5">
              {status ? (
                <>
                  branch <span className="font-mono">{status.branch}</span> · local{" "}
                  <span className="font-mono">{status.current_short}</span>
                  {status.update_available && (
                    <> → <span className="font-mono">{status.latest_short}</span></>
                  )}
                </>
              ) : (
                "checking…"
              )}
            </div>
            {status?.update_available && status.latest_subject && (
              <div className="text-xs text-text-muted mt-1">“{status.latest_subject}”</div>
            )}
          </div>
          <button
            type="button"
            onClick={check}
            disabled={checking}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg border border-border text-sm text-text-muted hover:text-text hover:bg-surface-hover disabled:opacity-60"
          >
            {checking ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <RotateCcw className="w-3.5 h-3.5" />}
            Check now
          </button>
        </div>

        <button
          type="button"
          onClick={apply}
          disabled={applying || !status?.update_available}
          className="w-full inline-flex items-center justify-center gap-2 px-3 py-2 rounded-lg bg-accent hover:bg-accent-hover text-white text-sm font-medium disabled:opacity-50"
        >
          {applying ? <Loader2 className="w-4 h-4 animate-spin" /> : <ArrowUpCircle className="w-4 h-4" />}
          {applying ? "Updating…" : "Update now"}
        </button>

        {msg && <p className="text-xs text-text-muted">{msg}</p>}
      </section>

      <section className="space-y-3">
        <h2 className="text-xs uppercase tracking-wide text-text-muted">Auto-check</h2>
        <div className="rounded-lg border border-border bg-surface p-4 space-y-3">
          <Toggle
            label="Notify when updates are available"
            hint="Shows a corner toast when new commits land on the remote."
            checked={p.updateCheckEnabled}
            onChange={(v) => p.set("updateCheckEnabled", v)}
          />
          <div className="flex items-center justify-between gap-4 border-t border-border/60 pt-3">
            <span className="text-sm">Check interval (minutes)</span>
            <input
              type="number"
              min={1}
              value={p.updateCheckIntervalMin}
              onChange={(e) => p.set("updateCheckIntervalMin", Math.max(1, Number(e.target.value) || 1))}
              className="w-20 px-2 py-1 rounded-md bg-bg border border-border text-sm text-right tabular-nums focus:border-border-hover outline-none"
            />
          </div>
        </div>
      </section>
    </div>
  );
}

const TABS = [
  { key: "appearance", label: "Appearance", icon: Palette },
  { key: "general", label: "General", icon: SlidersHorizontal },
  { key: "updates", label: "Updates", icon: ArrowUpCircle },
] as const;

export function Settings() {
  const [tab, setTab] = useState<(typeof TABS)[number]["key"]>("appearance");

  return (
    <div className="max-w-5xl mx-auto space-y-8">
      <nav className="flex gap-7 border-b border-border">
        {TABS.map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            type="button"
            onClick={() => setTab(key)}
            className={`inline-flex items-center gap-2 pt-1 pb-2.5 text-sm border-b-2 -mb-px transition-colors ${
              tab === key
                ? "border-accent text-text"
                : "border-transparent text-text-muted hover:text-text"
            }`}
          >
            <Icon className="w-4 h-4" />
            {label}
          </button>
        ))}
      </nav>

      {tab === "general" && <GeneralSettings />}
      {tab === "updates" && <UpdatesSettings />}
      {tab === "appearance" && <Appearance />}
    </div>
  );
}

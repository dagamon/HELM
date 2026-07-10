import { create } from "zustand";
import { persist } from "zustand/middleware";
import { api } from "@/api/client";
import type { Theme } from "@/api/types";

// Themes live as JSON files in the repo-root `themes/` folder and are served
// by `GET /api/themes`. The UI only picks one of them — there is no in-app
// editor; custom themes are added by dropping a JSON file into that folder.
//
// Each theme's `colors` map to the Tailwind v4 CSS variables `--color-{key}`
// declared in index.css `@theme`. Overriding the property on documentElement
// at runtime wins over the stylesheet `:root` rule, so every
// `bg-*`/`text-*`/`border-*` utility recolors live.
//
// `panels` is the per-theme palette of card background tints the dashboard
// may assign to service/stack cards (stored per card as the panel key).

export type ThemeColors = Record<string, string>;

export const DEFAULT_THEME_NAME = "helm-dark";

export const DEFAULT_COLORS: ThemeColors = {
  accent: "#3b82f6",
  "accent-hover": "#2563eb",
  bg: "#09090b",
  surface: "#111114",
  "surface-hover": "#19191e",
  "surface-raised": "#222228",
  border: "#1e1e24",
  text: "#ececef",
  "text-muted": "#8b8b93",
  success: "#34d399",
  warning: "#fbbf24",
  danger: "#f87171",
};

export function applyTheme(colors: ThemeColors) {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(colors)) {
    if (value) root.style.setProperty(`--color-${key}`, value);
  }
}

// Best-effort persistence to the server so the active theme is shared across
// browsers and configurable via the MCP server. Local storage stays the
// source of truth for instant, offline-safe paint.
interface ServerTheme {
  name?: string;
  /** Legacy shape from the old in-app palette editor. */
  activePreset?: string | null;
}

interface ThemeState {
  themes: Theme[];
  active: string;
  colors: ThemeColors;
  panels: Record<string, string>;
  loadThemes: () => Promise<void>;
  setTheme: (name: string) => void;
  hydrateFromServer: () => Promise<void>;
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      themes: [],
      active: DEFAULT_THEME_NAME,
      colors: DEFAULT_COLORS,
      panels: {},

      loadThemes: async () => {
        try {
          const themes = await api.listThemes();
          set({ themes });
          // Theme files may have changed on disk — refresh the active one.
          const current = themes.find((t) => t.name === get().active);
          if (current) {
            applyTheme(current.colors);
            set({ colors: current.colors, panels: current.panels ?? {} });
          }
        } catch {
          // server unreachable — keep the cached palette
        }
      },

      setTheme: (name) => {
        const theme = get().themes.find((t) => t.name === name);
        if (!theme) return;
        applyTheme(theme.colors);
        set({ active: name, colors: theme.colors, panels: theme.panels ?? {} });
        api.putSetting<ServerTheme>("theme", { name }).catch(() => {});
      },

      hydrateFromServer: async () => {
        await get().loadThemes();
        try {
          const remote = await api.getSetting<ServerTheme | null>("theme");
          const name = remote?.name ?? remote?.activePreset;
          if (name && name !== get().active) {
            const theme = get().themes.find((t) => t.name === name);
            if (theme) {
              applyTheme(theme.colors);
              set({
                active: name,
                colors: theme.colors,
                panels: theme.panels ?? {},
              });
            }
          }
        } catch {
          // server unreachable or unset — keep local theme
        }
      },
    }),
    {
      name: "helm-theme",
      partialize: (s) => ({
        active: s.active,
        colors: s.colors,
        panels: s.panels,
      }),
      onRehydrateStorage: () => (state) => {
        if (state) applyTheme(state.colors);
      },
    },
  ),
);

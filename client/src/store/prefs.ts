import { create } from "zustand";
import { persist } from "zustand/middleware";

// Client-side preferences persisted in the browser. These tune local UX bits
// that do not need server state: terminal rendering, update polling cadence,
// and destructive-action guarding.
export interface Prefs {
  terminalFontSize: number;
  logAutoScroll: boolean;
  confirmDestructive: boolean;
  updateCheckEnabled: boolean;
  updateCheckIntervalMin: number;
}

export const DEFAULT_PREFS: Prefs = {
  terminalFontSize: 13,
  logAutoScroll: true,
  confirmDestructive: true,
  updateCheckEnabled: true,
  updateCheckIntervalMin: 60,
};

interface PrefsState extends Prefs {
  set: <K extends keyof Prefs>(key: K, value: Prefs[K]) => void;
  reset: () => void;
}

export const usePrefs = create<PrefsState>()(
  persist(
    (set) => ({
      ...DEFAULT_PREFS,
      set: (key, value) => set({ [key]: value } as Partial<PrefsState>),
      reset: () => set({ ...DEFAULT_PREFS }),
    }),
    { name: "helm-prefs" },
  ),
);

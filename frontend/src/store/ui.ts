// UI-only state (Zustand): the selected increment and per-view filters.
// Server data lives in TanStack Query, never here.

import { create } from "zustand";

interface EpicFilters {
  owner: string | null;
  atRiskOnly: boolean;
  spilledOnly: boolean;
}

interface UiState {
  /** Selected increment; seeded from settings on first load. */
  incrementId: number | null;
  setIncrementId: (id: number | null) => void;

  epicFilters: EpicFilters;
  setEpicFilters: (patch: Partial<EpicFilters>) => void;

  syncing: boolean;
  syncDetail: string | null;
  setSyncState: (syncing: boolean, detail?: string | null) => void;
}

export const useUiStore = create<UiState>((set) => ({
  incrementId: null,
  setIncrementId: (id) => set({ incrementId: id }),

  epicFilters: { owner: null, atRiskOnly: false, spilledOnly: false },
  setEpicFilters: (patch) =>
    set((s) => ({ epicFilters: { ...s.epicFilters, ...patch } })),

  syncing: false,
  syncDetail: null,
  setSyncState: (syncing, detail = null) => set({ syncing, syncDetail: detail }),
}));

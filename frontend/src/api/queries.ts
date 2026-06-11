// TanStack Query hooks over the command layer. Query keys are scoped per
// increment; a successful sync invalidates everything for that increment.

import {
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";

import { api } from "./client";
import type { IncrementInput } from "./types";

export const keys = {
  settings: ["settings"] as const,
  projects: ["projects"] as const,
  dashboard: (inc: number) => ["dashboard", inc] as const,
  epics: (inc: number) => ["epics", inc] as const,
  epic: (inc: number, key: string) => ["epic", inc, key] as const,
  sprints: (inc: number) => ["sprints", inc] as const,
  sprint: (inc: number, id: number) => ["sprint", inc, id] as const,
  spillover: (inc: number) => ["spillover", inc] as const,
};

export function useSettings() {
  return useQuery({ queryKey: keys.settings, queryFn: api.getSettings });
}

export function useDashboard(incrementId: number | null) {
  return useQuery({
    queryKey: keys.dashboard(incrementId ?? -1),
    queryFn: () => api.getDashboard(incrementId!),
    enabled: incrementId != null,
  });
}

export function useEpics(incrementId: number | null) {
  return useQuery({
    queryKey: keys.epics(incrementId ?? -1),
    queryFn: () => api.getEpics(incrementId!),
    enabled: incrementId != null,
  });
}

export function useEpicDetail(incrementId: number | null, epicKey: string | undefined) {
  return useQuery({
    queryKey: keys.epic(incrementId ?? -1, epicKey ?? ""),
    queryFn: () => api.getEpicDetail(incrementId!, epicKey!),
    enabled: incrementId != null && !!epicKey,
  });
}

export function useSprints(incrementId: number | null) {
  return useQuery({
    queryKey: keys.sprints(incrementId ?? -1),
    queryFn: () => api.getSprints(incrementId!),
    enabled: incrementId != null,
  });
}

export function useSprintDetail(incrementId: number | null, sprintId: number | undefined) {
  return useQuery({
    queryKey: keys.sprint(incrementId ?? -1, sprintId ?? -1),
    queryFn: () => api.getSprintDetail(incrementId!, sprintId!),
    enabled: incrementId != null && sprintId != null,
  });
}

export function useSpillover(incrementId: number | null) {
  return useQuery({
    queryKey: keys.spillover(incrementId ?? -1),
    queryFn: () => api.getSpillover(incrementId!),
    enabled: incrementId != null,
  });
}

export function useSyncMutation() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (incrementId: number) => api.syncIncrement(incrementId),
    onSuccess: () => {
      // All read models may have changed.
      qc.invalidateQueries();
    },
  });
}

export function useSaveIncrement() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: IncrementInput) => api.saveIncrement(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: keys.settings }),
  });
}

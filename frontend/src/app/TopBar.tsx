// Global context bar: increment selector (scopes every screen), sync button
// with live progress, and the last-synced stamp.

import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";

import { api } from "@/api/client";
import { keys, useSettings, useSyncMutation } from "@/api/queries";
import type { SyncProgress } from "@/api/types";
import { useUiStore } from "@/store/ui";
import { useQueryClient } from "@tanstack/react-query";

export function TopBar() {
  const { data: settings } = useSettings();
  const { incrementId, setIncrementId, syncing, syncDetail, setSyncState } = useUiStore();
  const sync = useSyncMutation();
  const qc = useQueryClient();

  useEffect(() => {
    const unlisten = listen<SyncProgress>("sync://progress", (e) => {
      useUiStore.getState().setSyncState(true, e.payload.detail);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const onSelectIncrement = async (id: number) => {
    setIncrementId(id);
    await api.setActiveIncrement(id);
    qc.invalidateQueries({ queryKey: keys.settings });
  };

  const onSync = async () => {
    if (incrementId == null) return;
    setSyncState(true, "Starting sync…");
    try {
      await sync.mutateAsync(incrementId);
    } finally {
      setSyncState(false);
    }
  };

  const increments = settings?.increments ?? [];

  return (
    <header className="topbar">
      <select
        className="select"
        value={incrementId ?? ""}
        onChange={(e) => onSelectIncrement(Number(e.target.value))}
        disabled={increments.length === 0}
      >
        {increments.length === 0 && <option value="">No increments configured</option>}
        {increments.map((inc) => (
          <option key={inc.id} value={inc.id}>
            {inc.name}
          </option>
        ))}
      </select>
      {syncing && <span className="synced">{syncDetail ?? "Syncing…"}</span>}
      <div className="spacer" />
      {sync.isError && (
        <span className="synced" style={{ color: "var(--bad)" }}>
          Sync failed: {String(sync.error)}
        </span>
      )}
      <button className="btn" onClick={onSync} disabled={syncing || incrementId == null}>
        {syncing ? "Syncing…" : "Sync"}
      </button>
    </header>
  );
}

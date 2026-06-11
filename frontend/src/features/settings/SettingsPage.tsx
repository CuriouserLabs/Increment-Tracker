import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "@/api/client";
import { useSettings } from "@/api/queries";
import { ConnectionSection } from "./ConnectionSection";
import { FieldMappingSection } from "./FieldMappingSection";
import { IncrementsSection } from "./IncrementsSection";
import { ProjectsSection } from "./ProjectsSection";

export function SettingsPage() {
  const { data: settings, isLoading, error } = useSettings();
  const qc = useQueryClient();
  const [blocked, setBlocked] = useState<string | null>(null);
  const [clause, setClause] = useState<string | null>(null);

  if (isLoading) return <div className="empty">Loading settings…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;
  if (!settings) return null;

  const connected = !!settings.connection;
  const blockedValue = blocked ?? settings.blockedStatuses.join(", ");
  const clauseValue = clause ?? settings.epicChildrenClause ?? "";

  const saveAdvanced = async () => {
    await api.saveBlockedStatuses(
      blockedValue.split(",").map((s) => s.trim()).filter(Boolean),
    );
    await api.saveEpicChildrenClause(clauseValue.trim() || null);
    qc.invalidateQueries();
  };

  const clearData = async () => {
    if (!confirm("Clear all locally cached Jira data? Settings and credentials are kept.")) return;
    await api.clearLocalData();
    qc.invalidateQueries();
  };

  return (
    <div style={{ maxWidth: 820 }}>
      <h1 className="page-title">Settings</h1>

      <ConnectionSection settings={settings} />

      {connected && (
        <>
          <ProjectsSection settings={settings} />
          <IncrementsSection settings={settings} />
          <FieldMappingSection settings={settings} />

          <div className="card section">
            <h3>Advanced</h3>
            <div className="form-grid">
              <label>Blocked statuses</label>
              <input
                className="input"
                placeholder="Blocked, On Hold"
                value={blockedValue}
                onChange={(e) => setBlocked(e.target.value)}
              />
              <label>Epic children JQL</label>
              <input
                className="input"
                placeholder={'default: "Epic Link" in ({keys}) or parent in ({keys})'}
                value={clauseValue}
                onChange={(e) => setClause(e.target.value)}
              />
            </div>
            <p className="faint">
              The children clause template receives the epic keys as{" "}
              <code>{"{keys}"}</code>.
            </p>
            <button className="btn mt" onClick={saveAdvanced}>
              Save advanced settings
            </button>
          </div>

          <div className="card section">
            <h3>Local data</h3>
            <p className="muted">
              Issues, epics, sprints and snapshots are cached in SQLite so the app opens
              instantly and works offline. The PAT lives only in the OS keychain.
            </p>
            <button className="btn danger" onClick={clearData}>
              Clear local data
            </button>
          </div>
        </>
      )}
    </div>
  );
}

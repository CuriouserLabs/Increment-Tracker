import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "@/api/client";
import { useSettings } from "@/api/queries";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { ConnectionSection } from "./ConnectionSection";
import { FieldMappingSection } from "./FieldMappingSection";
import { IncrementsSection } from "./IncrementsSection";
import { ProjectsSection } from "./ProjectsSection";

export function SettingsPage() {
  const { data: settings, isLoading, error } = useSettings();
  const qc = useQueryClient();
  const [blocked, setBlocked] = useState<string | null>(null);
  const [clause, setClause] = useState<string | null>(null);
  const [sprintPattern, setSprintPattern] = useState<string | null>(null);
  const [sprintsPer, setSprintsPer] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  if (isLoading) return <div className="empty">Loading settings…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;
  if (!settings) return null;

  const connected = !!settings.connection;
  const blockedValue = blocked ?? settings.blockedStatuses.join(", ");
  const clauseValue = clause ?? settings.epicChildrenClause ?? "";
  const sprintPatternValue = sprintPattern ?? settings.sprintNaming.pattern;
  const sprintsPerValue = sprintsPer ?? String(settings.sprintNaming.sprintsPerIncrement);

  const saveAdvanced = async () => {
    await api.saveBlockedStatuses(
      blockedValue.split(",").map((s) => s.trim()).filter(Boolean),
    );
    await api.saveEpicChildrenClause(clauseValue.trim() || null);
    const perParsed = parseInt(sprintsPerValue, 10);
    await api.saveSprintNaming({
      pattern: sprintPatternValue.trim() || settings.sprintNaming.pattern,
      sprintsPerIncrement: Number.isFinite(perParsed) && perParsed > 0 ? perParsed : 6,
    });
    qc.invalidateQueries();
  };

  const clearData = async () => {
    setConfirmClear(false);
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
              <label>Sprint name pattern</label>
              <input
                className="input"
                placeholder={"(\\d+)\\s*:\\s*(\\d+)"}
                value={sprintPatternValue}
                onChange={(e) => setSprintPattern(e.target.value)}
              />
              <label>Sprints per increment</label>
              <input
                className="input"
                type="number"
                min={1}
                placeholder="6"
                value={sprintsPerValue}
                onChange={(e) => setSprintsPer(e.target.value)}
              />
            </div>
            <p className="faint">
              The children clause template receives the epic keys as{" "}
              <code>{"{keys}"}</code>.
            </p>
            <p className="faint">
              Spilled issues carry old sprints from previous increments. The sprint
              pattern identifies which increment a sprint belongs to — a regex with two
              capture groups (increment number, then sprint number). The default matches
              names like <code>Pegasus 25:2</code> (increment 25, sprint 2); only sprints
              of the increment in view are shown as its sprints.
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
            <button className="btn danger" onClick={() => setConfirmClear(true)}>
              Clear local data
            </button>
          </div>
        </>
      )}

      <ConfirmDialog
        open={confirmClear}
        title="Clear local data?"
        message="This removes all locally cached Jira data — issues, epics, sprints and snapshots. Your settings and credentials are kept, and the next sync re-fetches everything."
        confirmLabel="Clear data"
        danger
        onConfirm={clearData}
        onCancel={() => setConfirmClear(false)}
      />
    </div>
  );
}

// Increment definitions: name + JQL + dates. JQL is the configurability
// mechanism — Validate runs it against Jira and previews matched epics.

import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "@/api/client";
import { keys, useSaveIncrement } from "@/api/queries";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import type { Increment, JqlValidation, SettingsView } from "@/api/types";

const JQL_TEMPLATE = (projects: string[], name: string) =>
  `project in (${projects.length ? projects.join(", ") : "ABC"}) AND fixVersion = "${name || "Increment 25"}"`;

export function IncrementsSection({ settings }: { settings: SettingsView }) {
  const qc = useQueryClient();
  const save = useSaveIncrement();
  const [editing, setEditing] = useState<Partial<Increment> | null>(null);
  const [validation, setValidation] = useState<JqlValidation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<Increment | null>(null);

  const startNew = () =>
    setEditing({
      name: "",
      jql: JQL_TEMPLATE(settings.projects, ""),
      startDate: new Date().toISOString().slice(0, 10),
      endDate: new Date(Date.now() + 90 * 86400_000).toISOString().slice(0, 10),
    });

  const validate = async () => {
    if (!editing?.jql) return;
    setBusy(true);
    setError(null);
    try {
      setValidation(await api.validateJql(editing.jql));
    } catch (e) {
      setError(String(e));
      setValidation(null);
    } finally {
      setBusy(false);
    }
  };

  const submit = async () => {
    if (!editing?.name || !editing.jql || !editing.startDate || !editing.endDate) return;
    setError(null);
    try {
      await save.mutateAsync({
        id: editing.id ?? null,
        name: editing.name,
        jql: editing.jql,
        startDate: editing.startDate,
        endDate: editing.endDate,
      });
      setEditing(null);
      setValidation(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const remove = async () => {
    if (!pendingDelete) return;
    const id = pendingDelete.id;
    setPendingDelete(null);
    await api.deleteIncrement(id);
    qc.invalidateQueries({ queryKey: keys.settings });
  };

  return (
    <div className="card section">
      <h3>Increments</h3>
      {settings.increments.map((inc) => (
        <div key={inc.id} className="row" style={{ padding: "6px 0", gap: 10 }}>
          <strong>{inc.name}</strong>
          {inc.isActive && <span className="badge done">active</span>}
          <span className="faint">
            {inc.startDate} → {inc.endDate}
          </span>
          <span className="spacer" style={{ flex: 1 }} />
          <button className="btn secondary small" onClick={() => { setEditing(inc); setValidation(null); }}>
            Edit
          </button>
          <button className="btn danger small" onClick={() => setPendingDelete(inc)}>
            Delete
          </button>
        </div>
      ))}
      {settings.increments.length === 0 && !editing && (
        <p className="muted">No increments yet — define one to start tracking.</p>
      )}

      {editing ? (
        <div className="mt">
          {error && <div className="error-banner">{error}</div>}
          <div className="form-grid">
            <label>Name</label>
            <input
              className="input"
              placeholder="Increment 25"
              value={editing.name ?? ""}
              onChange={(e) => setEditing({ ...editing, name: e.target.value })}
            />
            <label>Epic JQL</label>
            <textarea
              className="textarea"
              value={editing.jql ?? ""}
              onChange={(e) => setEditing({ ...editing, jql: e.target.value })}
            />
            <label>Start date</label>
            <input
              className="input"
              type="date"
              value={editing.startDate ?? ""}
              onChange={(e) => setEditing({ ...editing, startDate: e.target.value })}
            />
            <label>End date</label>
            <input
              className="input"
              type="date"
              value={editing.endDate ?? ""}
              onChange={(e) => setEditing({ ...editing, endDate: e.target.value })}
            />
          </div>

          {validation && (
            <div className="notice-banner mt">
              Matches <strong>{validation.total}</strong> epic(s).{" "}
              {validation.notice && <em>{validation.notice} </em>}
              {validation.sample.length > 0 && (
                <span className="faint">
                  e.g. {validation.sample.map((s) => s.key).join(", ")}
                </span>
              )}
            </div>
          )}

          <div className="row mt">
            <button className="btn secondary" onClick={validate} disabled={busy || !editing.jql}>
              {busy ? "Validating…" : "Validate JQL"}
            </button>
            <button className="btn" onClick={submit} disabled={save.isPending}>
              Save increment
            </button>
            <button className="btn secondary" onClick={() => { setEditing(null); setValidation(null); }}>
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <button className="btn mt" onClick={startNew}>
          + New increment
        </button>
      )}

      <ConfirmDialog
        open={!!pendingDelete}
        title="Delete increment?"
        message={
          <>
            Delete <strong>{pendingDelete?.name}</strong> and its cached data? This
            can't be undone, but you can recreate the increment and re-sync.
          </>
        }
        confirmLabel="Delete"
        danger
        onConfirm={remove}
        onCancel={() => setPendingDelete(null)}
      />
    </div>
  );
}

// Project multi-select. Deselecting a project is a cache-hygiene action —
// its data is excluded from new increment JQL templates.

import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";

import { api } from "@/api/client";
import { keys } from "@/api/queries";
import type { SettingsView } from "@/api/types";

export function ProjectsSection({ settings }: { settings: SettingsView }) {
  const qc = useQueryClient();
  const [enabled, setEnabled] = useState(false);
  const { data: projects = [], isFetching, error } = useQuery({
    queryKey: keys.projects,
    queryFn: api.listProjects,
    enabled,
  });
  const selected = new Set(settings.projects);

  const toggle = async (key: string) => {
    const next = new Set(selected);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    await api.saveProjects([...next].sort());
    qc.invalidateQueries({ queryKey: keys.settings });
  };

  return (
    <div className="card section">
      <h3>Projects</h3>
      {settings.projects.length > 0 && (
        <p className="muted">Selected: {settings.projects.join(", ")}</p>
      )}
      {error != null && <div className="error-banner">{String(error)}</div>}
      {!enabled ? (
        <button className="btn secondary" onClick={() => setEnabled(true)}>
          Load projects from Jira
        </button>
      ) : isFetching ? (
        <p className="muted">Loading projects…</p>
      ) : (
        <div className="checks">
          {projects.map((p) => (
            <label key={p.key} className={`chip ${selected.has(p.key) ? "active" : ""}`}>
              <input
                type="checkbox"
                style={{ display: "none" }}
                checked={selected.has(p.key)}
                onChange={() => toggle(p.key)}
              />
              {p.key} — {p.name}
            </label>
          ))}
        </div>
      )}
    </div>
  );
}

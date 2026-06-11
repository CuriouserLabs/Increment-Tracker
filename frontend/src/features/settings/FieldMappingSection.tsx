// Custom-field mapping: auto-discovered, user-overridable. Instances with
// several "Story Points" fields exist; this is the escape hatch.

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "@/api/client";
import { keys } from "@/api/queries";
import type { FieldMapping, SettingsView } from "@/api/types";

const EMPTY: FieldMapping = {
  storyPoints: null,
  epicLink: null,
  sprint: null,
  epicStart: null,
  epicEnd: null,
};

const FIELDS: { key: keyof FieldMapping; label: string; hint: string }[] = [
  { key: "storyPoints", label: "Story Points", hint: "customfield_10016" },
  { key: "sprint", label: "Sprint", hint: "customfield_10020" },
  { key: "epicLink", label: "Epic Link", hint: "customfield_10014 (empty on Cloud = use parent)" },
  { key: "epicStart", label: "Epic start date", hint: "optional" },
  { key: "epicEnd", label: "Epic end date", hint: "duedate" },
];

export function FieldMappingSection({ settings }: { settings: SettingsView }) {
  const qc = useQueryClient();
  const [mapping, setMapping] = useState<FieldMapping>(EMPTY);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (settings.fieldMapping) setMapping(settings.fieldMapping);
  }, [settings.fieldMapping]);

  const discover = async () => {
    setBusy(true);
    setError(null);
    try {
      setMapping(await api.discoverFields());
      qc.invalidateQueries({ queryKey: keys.settings });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.saveFieldMapping(mapping);
      qc.invalidateQueries({ queryKey: keys.settings });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="card section">
      <h3>Field mapping</h3>
      {error && <div className="error-banner">{error}</div>}
      <div className="form-grid">
        {FIELDS.map((f) => (
          <FieldRow
            key={f.key}
            label={f.label}
            hint={f.hint}
            value={mapping[f.key] ?? ""}
            onChange={(v) => setMapping({ ...mapping, [f.key]: v || null })}
          />
        ))}
      </div>
      <div className="row mt">
        <button className="btn secondary" onClick={discover} disabled={busy}>
          {busy ? "Working…" : "Auto-discover"}
        </button>
        <button className="btn" onClick={save} disabled={busy}>
          Save mapping
        </button>
      </div>
    </div>
  );
}

function FieldRow({
  label,
  hint,
  value,
  onChange,
}: {
  label: string;
  hint: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <>
      <label>{label}</label>
      <input
        className="input"
        placeholder={hint}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      />
    </>
  );
}

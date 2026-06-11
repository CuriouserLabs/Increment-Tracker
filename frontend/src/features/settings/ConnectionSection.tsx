// Jira connection form. The PAT field is write-only: it can be set and
// tested but never read back (it lives in the OS keychain, not here).

import { useEffect, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";

import { api } from "@/api/client";
import { keys } from "@/api/queries";
import type { SettingsView } from "@/api/types";

export function ConnectionSection({ settings }: { settings: SettingsView }) {
  const qc = useQueryClient();
  const [baseUrl, setBaseUrl] = useState("");
  const [username, setUsername] = useState("");
  const [pat, setPat] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (settings.connection) {
      setBaseUrl(settings.connection.baseUrl);
      setUsername(settings.connection.username);
    }
  }, [settings.connection]);

  const test = async () => {
    setBusy(true);
    setError(null);
    setStatus(null);
    try {
      const r = await api.testConnection({ baseUrl, username, pat });
      setStatus(
        `Connected as ${r.displayName} (${r.authMode === "bearer" ? "Data Center PAT" : "Cloud API token"})`,
      );
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
      await api.saveConnection({ baseUrl, username, pat: pat || null, authMode: null });
      setPat("");
      setStatus("Connection saved. PAT stored in the OS keychain.");
      qc.invalidateQueries({ queryKey: keys.settings });
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="card section">
      <h3>Jira connection</h3>
      {error && <div className="error-banner">{error}</div>}
      {status && <div className="notice-banner">{status}</div>}
      <div className="form-grid">
        <label>Base URL</label>
        <input
          className="input"
          placeholder="https://jira.your-company.com"
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
        />
        <label>Username / email</label>
        <input
          className="input"
          placeholder="you@company.com"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
        />
        <label>Personal Access Token</label>
        <input
          className="input"
          type="password"
          placeholder={settings.connection?.hasPat ? "•••••• (stored — enter to replace)" : "paste token"}
          value={pat}
          onChange={(e) => setPat(e.target.value)}
        />
      </div>
      <div className="row mt">
        <button className="btn secondary" onClick={test} disabled={busy || !baseUrl || !username || !pat}>
          Test connection
        </button>
        <button
          className="btn"
          onClick={save}
          disabled={busy || !baseUrl || !username || (!pat && !settings.connection?.hasPat)}
        >
          Save
        </button>
      </div>
    </div>
  );
}

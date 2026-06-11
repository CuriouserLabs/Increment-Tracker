// The leadership view: carried-forward SP, chronic offenders, every spilled
// issue (worst first), and epics carried across increments.

import { useSettings, useSpillover } from "@/api/queries";
import type { SpilledIssueRow } from "@/api/types";
import { Badge } from "@/components/Badge";
import { DataTable, type Column } from "@/components/DataTable";
import { KpiCard } from "@/components/KpiCard";
import { StatusChip } from "@/components/StatusChip";
import { fmtPct, fmtSp } from "@/lib/format";
import { openInJira } from "@/lib/jira";
import { useUiStore } from "@/store/ui";

export function SpilloverPage() {
  const { incrementId } = useUiStore();
  const { data, isLoading, error } = useSpillover(incrementId);
  const { data: settings } = useSettings();

  if (isLoading) return <div className="empty">Loading spillover report…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;
  if (!data) return null;

  const baseUrl = settings?.connection?.baseUrl;

  const columns: Column<SpilledIssueRow>[] = [
    { key: "key", header: "Key", width: "110px", render: (r) => <span className="key">{r.issue.key}</span> },
    { key: "summary", header: "Summary", render: (r) => r.issue.summary },
    {
      key: "sp",
      header: "SP",
      align: "right",
      width: "60px",
      render: (r) => fmtSp(r.issue.effectiveSp),
    },
    {
      key: "status",
      header: "Status",
      width: "120px",
      render: (r) => <StatusChip category={r.issue.statusCategory} label={r.issue.status} />,
    },
    {
      key: "spills",
      header: "Spilled in",
      render: (r) => (
        <span className="row" style={{ gap: 4, flexWrap: "wrap" }}>
          <Badge kind="spill" count={r.issue.spillCount} />
          <span className="faint">{r.sprintNames.join(" → ")}</span>
        </span>
      ),
    },
  ];

  const worstSprint = [...data.perSprint]
    .filter((s) => s.state === "closed")
    .sort((a, b) => b.spilloverRate - a.spilloverRate)[0];

  return (
    <div>
      <h1 className="page-title">Spillover</h1>

      <div className="kpi-strip">
        <KpiCard
          label="Carried forward"
          value={`${fmtSp(data.carriedForwardSp)} SP`}
          sub="committed earlier, still open"
          tone={data.carriedForwardSp > 0 ? "warn" : "good"}
        />
        <KpiCard
          label="Chronic spillers"
          value={String(data.chronic.length)}
          sub="spilled across 2+ sprints"
          tone={data.chronic.length > 0 ? "bad" : "good"}
        />
        <KpiCard
          label="Worst sprint"
          value={worstSprint ? worstSprint.name : "—"}
          sub={worstSprint ? `${fmtPct(worstSprint.spilloverRate)} spillover` : "no closed sprints"}
        />
        <KpiCard
          label="Carried epics"
          value={String(data.carriedEpics.length)}
          sub="from previous increments"
          tone={data.carriedEpics.length > 0 ? "warn" : "good"}
        />
      </div>

      {data.chronic.length > 0 && (
        <div className="card mb" style={{ padding: 0, borderColor: "var(--bad)" }}>
          <h3 style={{ padding: "12px 16px 0", color: "var(--bad)" }}>Chronic offenders</h3>
          <DataTable
            columns={columns}
            rows={data.chronic}
            rowKey={(r) => r.issue.key}
            onRowClick={(r) => openInJira(baseUrl, r.issue.key)}
          />
        </div>
      )}

      <div className="card mb" style={{ padding: 0 }}>
        <h3 style={{ padding: "12px 16px 0" }}>All spilled issues — worst first</h3>
        <DataTable
          columns={columns}
          rows={data.all}
          rowKey={(r) => r.issue.key}
          onRowClick={(r) => openInJira(baseUrl, r.issue.key)}
          emptyMessage="No spillover. Either the team is crushing it, or nothing has been synced yet."
        />
      </div>

      {data.carriedEpics.length > 0 && (
        <div className="card">
          <h3>Epics carried across increments</h3>
          {data.carriedEpics.map((e) => (
            <div key={e.key} className="row" style={{ padding: "6px 0", gap: 8 }}>
              <span className="key">{e.key}</span>
              <span>{e.name}</span>
              <Badge kind="carried" title={`from ${e.carriedFrom}`} label={`from ${e.carriedFrom}`} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// Epics table: key · name · owner · SP progress · pace · sprint span ·
// spill · badges. Filters as visible chips.

import { useMemo } from "react";
import { useNavigate } from "react-router-dom";

import { useEpics } from "@/api/queries";
import type { EpicListRow } from "@/api/types";
import { Badge } from "@/components/Badge";
import { DataTable, type Column } from "@/components/DataTable";
import { FilterChips } from "@/components/FilterChips";
import { ProgressBar } from "@/components/ProgressBar";
import { useUiStore } from "@/store/ui";

export function EpicsPage() {
  const { incrementId, epicFilters, setEpicFilters } = useUiStore();
  const { data: rows = [], isLoading, error } = useEpics(incrementId);
  const navigate = useNavigate();

  const owners = useMemo(
    () => [...new Set(rows.map((r) => r.epic.owner).filter((o): o is string => !!o))].sort(),
    [rows],
  );

  const filtered = rows.filter((r) => {
    if (epicFilters.atRiskOnly && !r.atRisk) return false;
    if (epicFilters.spilledOnly && r.spillCount === 0) return false;
    if (epicFilters.owner && r.epic.owner !== epicFilters.owner) return false;
    return true;
  });

  const columns: Column<EpicListRow>[] = [
    { key: "key", header: "Epic", width: "110px", render: (r) => <span className="key">{r.epic.key}</span> },
    {
      key: "name",
      header: "Name",
      render: (r) => (
        <span className="row" style={{ gap: 6, flexWrap: "wrap" }}>
          {r.epic.name}
          {r.atRisk && <Badge kind="risk" />}
          {r.epic.carriedFrom && <Badge kind="carried" title={`from ${r.epic.carriedFrom}`} />}
          {r.epic.removedFromPlan && <Badge kind="removed" />}
          {r.breakdown.unestimatedCount > 0 && (
            <Badge kind="unestimated" label={`${r.breakdown.unestimatedCount} unestimated`} />
          )}
        </span>
      ),
    },
    { key: "owner", header: "Owner", width: "140px", render: (r) => r.epic.owner ?? "—" },
    {
      key: "progress",
      header: "Progress (SP)",
      width: "240px",
      render: (r) => (
        <ProgressBar
          done={r.breakdown.doneSp}
          inProgress={r.breakdown.inProgressSp}
          total={r.breakdown.totalSp}
        />
      ),
    },
    {
      key: "pace",
      header: "Pace",
      width: "80px",
      align: "right",
      render: (r) => {
        const pct = Math.round(r.pace * 100);
        const color = pct < -15 ? "var(--bad)" : pct < 0 ? "var(--warn)" : "var(--good)";
        return (
          <span style={{ color }}>
            {pct >= 0 ? "▲" : "▼"} {Math.abs(pct)}%
          </span>
        );
      },
    },
    {
      key: "span",
      header: "Sprints",
      width: "140px",
      render: (r) => <span className="faint">{r.sprintSpan ?? "—"}</span>,
    },
    {
      key: "spill",
      header: "Spill",
      width: "100px",
      render: (r) => (r.spillCount > 0 ? <Badge kind="spill" label={`${r.spillCount}`} /> : null),
    },
  ];

  if (isLoading) return <div className="empty">Loading epics…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;

  return (
    <div>
      <h1 className="page-title">Epics</h1>
      <div className="row mb" style={{ flexWrap: "wrap" }}>
        <FilterChips
          options={[
            { id: "atRisk", label: "⚠ At risk only", active: epicFilters.atRiskOnly },
            { id: "spilled", label: "⮔ Spilled only", active: epicFilters.spilledOnly },
          ]}
          onToggle={(id) =>
            id === "atRisk"
              ? setEpicFilters({ atRiskOnly: !epicFilters.atRiskOnly })
              : setEpicFilters({ spilledOnly: !epicFilters.spilledOnly })
          }
        />
        <select
          className="select"
          value={epicFilters.owner ?? ""}
          onChange={(e) => setEpicFilters({ owner: e.target.value || null })}
        >
          <option value="">All owners</option>
          {owners.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>
      </div>
      <div className="card" style={{ padding: 0 }}>
        <DataTable
          columns={columns}
          rows={filtered}
          rowKey={(r) => r.epic.key}
          onRowClick={(r) => navigate(`/epics/${r.epic.key}`)}
          emptyMessage="No epics match the current filters. Sync the increment or adjust filters."
        />
      </div>
    </div>
  );
}

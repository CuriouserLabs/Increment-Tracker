// Sprint drill-down: spilled work on top (it's the signal), then committed
// and mid-sprint additions.

import { useNavigate, useParams } from "react-router-dom";

import { useDashboard, useSettings, useSprintDetail } from "@/api/queries";
import { IssueTable } from "@/components/IssueTable";
import { KpiCard } from "@/components/KpiCard";
import { fmtDateLong, fmtPct, fmtSp } from "@/lib/format";
import { openInJira } from "@/lib/jira";
import { useUiStore } from "@/store/ui";

export function SprintDetailPage() {
  const { sprintId } = useParams();
  const { incrementId } = useUiStore();
  const { data, isLoading, error } = useSprintDetail(
    incrementId,
    sprintId ? Number(sprintId) : undefined,
  );
  const { data: settings } = useSettings();
  const { data: dashboard } = useDashboard(incrementId);
  const navigate = useNavigate();

  if (isLoading) return <div className="empty">Loading sprint…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;
  if (!data) return null;

  const baseUrl = settings?.connection?.baseUrl;
  const open = (key: string) => openInJira(baseUrl, key);
  const sprints = dashboard?.sprints ?? [data.sprint];

  return (
    <div>
      <button className="btn secondary small mb" onClick={() => navigate("/sprints")}>
        ← Sprints
      </button>

      <h1 className="page-title">
        {data.sprint.name}
        <small>
          {data.sprint.state} · {fmtDateLong(data.sprint.startDate)} →{" "}
          {fmtDateLong(data.sprint.endDate)}
        </small>
      </h1>

      <div className="kpi-strip">
        <KpiCard label="Committed" value={`${fmtSp(data.stats.committedSp)} SP`} sub={`${data.stats.committedCount} issues`} />
        <KpiCard label="Done" value={`${fmtSp(data.stats.doneSp)} SP`} sub={`completion ${fmtPct(data.stats.completionRate)}`} tone="good" />
        <KpiCard
          label="Spilled"
          value={`${fmtSp(data.stats.spilledSp)} SP`}
          sub={`spillover rate ${fmtPct(data.stats.spilloverRate)}`}
          tone={data.stats.spilledSp > 0 ? "bad" : "good"}
        />
        <KpiCard label="Added mid-sprint" value={`+${fmtSp(data.stats.addedSp)} SP`} sub="excluded from commitment" />
      </div>

      {data.spilled.length > 0 && (
        <div className="card mb" style={{ padding: 0, borderColor: "var(--bad)" }}>
          <h3 style={{ padding: "12px 16px 0", color: "var(--bad)" }}>
            ⮔ Spilled out of this sprint
          </h3>
          <IssueTable issues={data.spilled} sprints={sprints} onOpen={open} />
        </div>
      )}

      <div className="card mb" style={{ padding: 0 }}>
        <h3 style={{ padding: "12px 16px 0" }}>Committed at sprint start</h3>
        <IssueTable
          issues={data.committed}
          sprints={sprints}
          onOpen={open}
          emptyMessage="Nothing was committed to this sprint."
        />
      </div>

      {data.added.length > 0 && (
        <div className="card mb" style={{ padding: 0 }}>
          <h3 style={{ padding: "12px 16px 0" }}>＋ Added mid-sprint</h3>
          <IssueTable issues={data.added} sprints={sprints} onOpen={open} />
        </div>
      )}
    </div>
  );
}

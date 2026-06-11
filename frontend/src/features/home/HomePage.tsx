// Home = the increment dashboard. Four zones: KPI strip, insights, the epic
// Gantt, then burn-up + sprint completion side by side.

import { useNavigate } from "react-router-dom";

import { useDashboard, useSettings } from "@/api/queries";
import { EmptyState } from "@/components/EmptyState";
import { KpiCard } from "@/components/KpiCard";
import { fmtPct, fmtSp, timeAgo } from "@/lib/format";
import { useUiStore } from "@/store/ui";
import { BurnupChart } from "./BurnupChart";
import { GanttChart } from "./GanttChart";
import { InsightsPanel } from "./InsightsPanel";
import { SprintCompletionChart } from "./SprintCompletionChart";

export function HomePage() {
  const { incrementId } = useUiStore();
  const { data: settings } = useSettings();
  const { data, isLoading, error } = useDashboard(incrementId);
  const navigate = useNavigate();

  if (!settings?.connection) {
    return (
      <EmptyState
        title="Welcome to Increment Tracker"
        message="Connect to Jira and define an increment to get started."
        action={
          <button className="btn" onClick={() => navigate("/settings")}>
            Open Settings
          </button>
        }
      />
    );
  }
  if (incrementId == null) {
    return (
      <EmptyState
        title="No increment selected"
        message="Create an increment in Settings, then sync."
        action={
          <button className="btn" onClick={() => navigate("/settings")}>
            Open Settings
          </button>
        }
      />
    );
  }
  if (isLoading) return <div className="empty">Loading dashboard…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;
  if (!data) return null;

  const { kpis } = data;
  const variancePct = Math.round(kpis.variance * 100);
  const never = data.lastSynced == null;

  if (never) {
    return (
      <EmptyState
        title={`${data.increment.name} has not been synced yet`}
        message="Press Sync in the top bar to pull epics and issues from Jira."
      />
    );
  }

  return (
    <div>
      <h1 className="page-title">
        {data.increment.name}
        <small>synced {timeAgo(data.lastSynced)}</small>
      </h1>

      <div className="kpi-strip">
        <KpiCard
          label="Progress"
          value={fmtPct(kpis.progress)}
          sub={`${fmtSp(kpis.doneSp)}/${fmtSp(kpis.totalSp)} SP · ${fmtSp(kpis.inProgressSp)} in flight`}
          onClick={() => navigate("/epics")}
        />
        <KpiCard
          label="Pace vs plan"
          value={`${variancePct >= 0 ? "+" : ""}${variancePct}%`}
          sub={`expected ${fmtPct(kpis.expectedProgress)} by today`}
          tone={variancePct < -10 ? "bad" : variancePct < 0 ? "warn" : "good"}
          onClick={() => navigate("/epics")}
        />
        <KpiCard
          label="Carried forward"
          value={`${fmtSp(kpis.carriedForwardSp)} SP`}
          sub={
            kpis.chronicSpillCount > 0
              ? `${kpis.chronicSpillCount} chronic spiller(s)`
              : "no chronic spillers"
          }
          tone={kpis.carriedForwardSp > 0 ? "warn" : "good"}
          onClick={() => navigate("/spillover")}
        />
        <KpiCard
          label="Scope change"
          value={`+${fmtSp(kpis.scopeAddedSp)} / −${fmtSp(kpis.scopeRemovedSp)}`}
          sub={
            kpis.descopedSp > 0
              ? `${fmtSp(kpis.descopedSp)} SP descoped`
              : "since first sync"
          }
          onClick={() => navigate("/epics")}
        />
      </div>

      <InsightsPanel insights={data.insights} />

      <div className="section">
        <GanttChart increment={data.increment} rows={data.gantt} sprints={data.sprints} />
      </div>

      <div className="grid-2">
        <BurnupChart points={data.burnup} />
        <SprintCompletionChart points={data.sprintCompletion} />
      </div>
    </div>
  );
}

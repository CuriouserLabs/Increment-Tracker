// Epic drill-down: header with progress + badges, child issues grouped by
// sprint (toggle: by status), descoped drawer, "Open in Jira" deep link.

import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useEpicDetail, useSettings } from "@/api/queries";
import type { Issue } from "@/api/types";
import { Badge } from "@/components/Badge";
import { IssueTable } from "@/components/IssueTable";
import { ProgressBar } from "@/components/ProgressBar";
import { fmtDateLong, fmtPct, fmtSp } from "@/lib/format";
import { openInJira } from "@/lib/jira";
import { useUiStore } from "@/store/ui";

type GroupBy = "sprint" | "status";

export function EpicDetailPage() {
  const { epicKey } = useParams();
  const { incrementId } = useUiStore();
  const { data, isLoading, error } = useEpicDetail(incrementId, epicKey);
  const { data: settings } = useSettings();
  const [groupBy, setGroupBy] = useState<GroupBy>("sprint");
  const navigate = useNavigate();

  if (isLoading) return <div className="empty">Loading epic…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;
  if (!data) return null;

  const baseUrl = settings?.connection?.baseUrl;
  const open = (key: string) => openInJira(baseUrl, key);

  const groups: { title: string; issues: Issue[] }[] = [];
  if (groupBy === "sprint") {
    for (const sprint of data.sprints) {
      const issues = data.issues.filter((i) => i.currentSprintId === sprint.id);
      if (issues.length > 0) groups.push({ title: sprint.name, issues });
    }
    const backlog = data.issues.filter((i) => i.currentSprintId == null);
    if (backlog.length > 0) groups.push({ title: "No sprint", issues: backlog });
  } else {
    for (const cat of ["in_progress", "new", "done"] as const) {
      const issues = data.issues.filter((i) => i.statusCategory === cat);
      if (issues.length > 0) {
        groups.push({
          title: cat === "in_progress" ? "In progress" : cat === "new" ? "Not started" : "Done",
          issues,
        });
      }
    }
  }

  return (
    <div>
      <button className="btn secondary small mb" onClick={() => navigate("/epics")}>
        ← Epics
      </button>

      <div className="card mb">
        <div className="row" style={{ justifyContent: "space-between", flexWrap: "wrap" }}>
          <div>
            <h1 className="page-title" style={{ marginBottom: 4 }}>
              {data.epic.name}
              <small className="key">{data.epic.key}</small>
            </h1>
            <div className="row" style={{ gap: 8, flexWrap: "wrap" }}>
              <span className="faint">Owner: {data.epic.owner ?? "—"}</span>
              <span className="faint">
                {fmtDateLong(data.epic.startDate)} → {fmtDateLong(data.epic.endDate)}
              </span>
              {data.atRisk && <Badge kind="risk" />}
              {data.epic.carriedFrom && (
                <Badge kind="carried" title={`carried from ${data.epic.carriedFrom}`} />
              )}
              {data.epic.removedFromPlan && <Badge kind="removed" />}
            </div>
          </div>
          <button className="btn secondary" onClick={() => open(data.epic.key)}>
            Open in Jira ↗
          </button>
        </div>
        <div className="mt">
          <ProgressBar
            done={data.breakdown.doneSp}
            inProgress={data.breakdown.inProgressSp}
            total={data.breakdown.totalSp}
          />
          <div className="faint" style={{ marginTop: 6 }}>
            expected {fmtPct(data.expectedProgress)} by today ·{" "}
            {fmtSp(data.breakdown.blockedSp)} SP blocked ·{" "}
            {data.breakdown.unestimatedCount} unestimated issue(s)
          </div>
        </div>
      </div>

      <div className="row mb">
        <span className="faint">Group by</span>
        <select className="select" value={groupBy} onChange={(e) => setGroupBy(e.target.value as GroupBy)}>
          <option value="sprint">Sprint</option>
          <option value="status">Status</option>
        </select>
      </div>

      {groups.map((g) => (
        <div className="card mb" key={g.title} style={{ padding: 0 }}>
          <h3 style={{ padding: "12px 16px 0" }}>{g.title}</h3>
          <IssueTable issues={g.issues} sprints={data.sprints} onOpen={open} />
        </div>
      ))}

      {data.descoped.length > 0 && (
        <details className="drawer card">
          <summary>Descoped ({data.descoped.length}) — removed from progress math</summary>
          <IssueTable issues={data.descoped} sprints={data.sprints} onOpen={open} />
        </details>
      )}
    </div>
  );
}

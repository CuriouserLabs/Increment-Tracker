// The centerpiece: one row per epic across the increment's sprint columns.
// Bar position = epic dates (fallback: full increment), fill = SP done %,
// hatched = in progress, vertical line = today. Fill lagging the today line
// IS the at-risk visualization.

import { useNavigate } from "react-router-dom";

import type { GanttRow, Increment, Sprint } from "@/api/types";
import { Badge } from "@/components/Badge";
import { fmtPct, timeFraction } from "@/lib/format";

interface Props {
  increment: Increment;
  rows: GanttRow[];
  sprints: Sprint[];
}

export function GanttChart({ increment, rows, sprints }: Props) {
  const navigate = useNavigate();
  const todayPct = timeFraction(increment.startDate, increment.endDate, new Date()) * 100;

  // Sprint columns: sprints overlapping the increment window, by start date.
  const columns = sprints
    .filter((s) => s.startDate && s.endDate)
    .filter(
      (s) =>
        new Date(s.endDate!) >= new Date(increment.startDate) &&
        new Date(s.startDate!) <= new Date(increment.endDate),
    )
    .sort((a, b) => new Date(a.startDate!).getTime() - new Date(b.startDate!).getTime());

  const barGeometry = (row: GanttRow) => {
    const left = row.startDate
      ? timeFraction(increment.startDate, increment.endDate, row.startDate) * 100
      : 0;
    const right = row.endDate
      ? timeFraction(increment.startDate, increment.endDate, row.endDate) * 100
      : 100;
    return { left, width: Math.max(right - left, 2) };
  };

  return (
    <div className="card">
      <h3>Epic timeline</h3>
      <div className="gantt">
        <div className="gantt-header">
          <div className="gantt-label-col faint">
            {rows.length} epics · sorted by size
          </div>
          <div className="gantt-track-col">
            {columns.length > 0 ? (
              columns.map((s) => (
                <div
                  key={s.id}
                  className="gantt-sprint-col"
                  title={`Open ${s.name}`}
                  onClick={() => navigate(`/sprints/${s.id}`)}
                >
                  {s.name}
                </div>
              ))
            ) : (
              <div className="gantt-sprint-col" style={{ cursor: "default" }}>
                {increment.startDate} → {increment.endDate}
              </div>
            )}
          </div>
        </div>

        <div style={{ position: "relative" }}>
          {rows.map((row) => {
            const geo = barGeometry(row);
            return (
              <div
                key={row.epicKey}
                className="gantt-row"
                onClick={() => navigate(`/epics/${row.epicKey}`)}
                title={`${row.epicKey} — ${fmtPct(row.progress)} done`}
              >
                <div className="gantt-label-col epic-label">
                  <span className="epic-name">{row.name}</span>
                  <span className="epic-meta">
                    <span className="key">{row.epicKey}</span>
                    {row.owner && <span>{row.owner}</span>}
                    {row.atRisk && <Badge kind="risk" />}
                    {row.carriedOver && <Badge kind="carried" />}
                    {row.removedFromPlan && <Badge kind="removed" />}
                    {row.noChildren && <Badge kind="nochildren" />}
                  </span>
                </div>
                <div className="gantt-track">
                  <div
                    className={`gantt-bar ${row.atRisk ? "at-risk" : ""}`}
                    style={{ left: `${geo.left}%`, width: `${geo.width}%` }}
                  >
                    <div className="fill-done" style={{ width: `${row.progress * 100}%` }} />
                    <div
                      className="fill-inprogress"
                      style={{
                        left: `${row.progress * 100}%`,
                        width: `${row.totalSp > 0 ? (row.inProgressSp / row.totalSp) * 100 : 0}%`,
                      }}
                    />
                    <span className="pct">{fmtPct(row.progress)}</span>
                  </div>
                </div>
              </div>
            );
          })}
          {/* Today line spans all rows, offset by the label column width. */}
          <div
            className="gantt-today"
            style={{ left: `calc(230px + (100% - 230px) * ${todayPct / 100})` }}
          />
        </div>
      </div>
    </div>
  );
}

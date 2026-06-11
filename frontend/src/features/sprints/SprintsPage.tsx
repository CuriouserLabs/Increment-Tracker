// Per-sprint completion: committed vs done vs spilled, with rates.

import { useNavigate } from "react-router-dom";

import { useSprints } from "@/api/queries";
import type { SprintCompletionPoint } from "@/api/types";
import { Badge } from "@/components/Badge";
import { DataTable, type Column } from "@/components/DataTable";
import { ProgressBar } from "@/components/ProgressBar";
import { fmtPct, fmtSp } from "@/lib/format";
import { useUiStore } from "@/store/ui";

export function SprintsPage() {
  const { incrementId } = useUiStore();
  const { data: sprints = [], isLoading, error } = useSprints(incrementId);
  const navigate = useNavigate();

  const columns: Column<SprintCompletionPoint>[] = [
    {
      key: "name",
      header: "Sprint",
      width: "160px",
      render: (s) => (
        <span className="row" style={{ gap: 6 }}>
          {s.name}
          {s.state === "active" && <Badge kind="done" label="active" />}
        </span>
      ),
    },
    {
      key: "completion",
      header: "Committed SP done",
      width: "260px",
      render: (s) => <ProgressBar done={s.doneSp} total={s.committedSp + s.addedSp} showLabel={false} />,
    },
    {
      key: "committed",
      header: "Committed",
      align: "right",
      render: (s) => `${fmtSp(s.committedSp)} SP`,
    },
    { key: "added", header: "Added", align: "right", render: (s) => `+${fmtSp(s.addedSp)} SP` },
    { key: "done", header: "Done", align: "right", render: (s) => `${fmtSp(s.doneSp)} SP` },
    {
      key: "spill",
      header: "Spilled",
      align: "right",
      render: (s) =>
        s.spilledSp > 0 ? (
          <span style={{ color: "var(--bad)" }}>
            {fmtSp(s.spilledSp)} SP ({fmtPct(s.spilloverRate)})
          </span>
        ) : (
          "—"
        ),
    },
    {
      key: "rate",
      header: "Completion",
      align: "right",
      render: (s) => fmtPct(s.completionRate),
    },
  ];

  if (isLoading) return <div className="empty">Loading sprints…</div>;
  if (error) return <div className="error-banner">{String(error)}</div>;

  return (
    <div>
      <h1 className="page-title">Sprints</h1>
      <div className="card" style={{ padding: 0 }}>
        <DataTable
          columns={columns}
          rows={sprints}
          rowKey={(s) => String(s.sprintId)}
          onRowClick={(s) => navigate(`/sprints/${s.sprintId}`)}
          emptyMessage="No sprint data yet — sync the increment first."
        />
      </div>
    </div>
  );
}

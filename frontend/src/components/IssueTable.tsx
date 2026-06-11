// Shared issue table used by epic detail, sprint detail and spillover views.

import type { Issue, Sprint } from "@/api/types";
import { fmtSp } from "@/lib/format";
import { Badge } from "./Badge";
import { DataTable, type Column } from "./DataTable";
import { StatusChip } from "./StatusChip";

interface Props {
  issues: Issue[];
  sprints?: Sprint[];
  onOpen?: (key: string) => void;
  emptyMessage?: string;
}

export function IssueTable({ issues, sprints = [], onOpen, emptyMessage }: Props) {
  const sprintName = (id: number | null) =>
    id == null ? "—" : sprints.find((s) => s.id === id)?.name ?? `Sprint ${id}`;

  const columns: Column<Issue>[] = [
    { key: "key", header: "Key", width: "110px", render: (i) => <span className="key">{i.key}</span> },
    { key: "summary", header: "Summary", render: (i) => i.summary },
    {
      key: "sp",
      header: "SP",
      align: "right",
      width: "60px",
      render: (i) =>
        i.sp == null ? (
          <span title={`Imputed at ${fmtSp(i.effectiveSp)} SP`}>∅</span>
        ) : (
          fmtSp(i.sp)
        ),
    },
    {
      key: "status",
      header: "Status",
      width: "120px",
      render: (i) => <StatusChip category={i.statusCategory} label={i.status} />,
    },
    {
      key: "sprint",
      header: "Sprint",
      width: "130px",
      render: (i) => <span className="faint">{sprintName(i.currentSprintId)}</span>,
    },
    {
      key: "flags",
      header: "Flags",
      width: "200px",
      render: (i) => (
        <span className="row" style={{ gap: 4, flexWrap: "wrap" }}>
          {i.spillCount >= 1 && <Badge kind="spill" count={i.spillCount} />}
          {i.blocked && <Badge kind="blocked" />}
          {i.reopened && <Badge kind="reopened" />}
        </span>
      ),
    },
  ];

  return (
    <DataTable
      columns={columns}
      rows={issues}
      rowKey={(i) => i.key}
      onRowClick={onOpen ? (i) => onOpen(i.key) : undefined}
      emptyMessage={emptyMessage}
    />
  );
}

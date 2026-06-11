// Per-sprint commitment reliability: done vs spilled (stacked), with
// mid-sprint additions as a separate thin bar. Click a bar to drill in.

import {
  Bar,
  BarChart,
  CartesianGrid,
  Legend,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { useNavigate } from "react-router-dom";

import type { SprintCompletionPoint } from "@/api/types";

export function SprintCompletionChart({ points }: { points: SprintCompletionPoint[] }) {
  const navigate = useNavigate();
  const open = (p: SprintCompletionPoint | undefined) => {
    if (p) navigate(`/sprints/${p.sprintId}`);
  };

  return (
    <div className="card">
      <h3>Sprint completion — done vs spilled</h3>
      <ResponsiveContainer width="100%" height={220}>
        <BarChart data={points} margin={{ top: 4, right: 12, bottom: 0, left: -16 }}>
          <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" />
          <XAxis dataKey="name" stroke="var(--text-faint)" fontSize={11} />
          <YAxis stroke="var(--text-faint)" fontSize={11} />
          <Tooltip
            cursor={{ fill: "rgba(255,255,255,0.04)" }}
            contentStyle={{
              background: "var(--bg-raised)",
              border: "1px solid var(--border)",
              borderRadius: 8,
            }}
            labelStyle={{ color: "var(--text)" }}
          />
          <Legend wrapperStyle={{ fontSize: 12 }} />
          <Bar
            dataKey="doneSp"
            name="Done"
            stackId="committed"
            fill="var(--good)"
            onClick={(d: unknown) => open(d as SprintCompletionPoint)}
          />
          <Bar
            dataKey="spilledSp"
            name="Spilled"
            stackId="committed"
            fill="var(--bad)"
            onClick={(d: unknown) => open(d as SprintCompletionPoint)}
          />
          <Bar dataKey="addedSp" name="Added mid-sprint" fill="var(--accent)" barSize={6} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

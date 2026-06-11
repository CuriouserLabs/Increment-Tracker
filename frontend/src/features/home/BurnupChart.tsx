// Burn-up (not burn-down): the rising scope line makes scope creep visible;
// dips in the done line are honest reopens.

import {
  CartesianGrid,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import type { BurnupPoint } from "@/api/types";

export function BurnupChart({ points }: { points: BurnupPoint[] }) {
  return (
    <div className="card">
      <h3>Burn-up — done vs scope</h3>
      <ResponsiveContainer width="100%" height={220}>
        <LineChart data={points} margin={{ top: 4, right: 12, bottom: 0, left: -16 }}>
          <CartesianGrid stroke="var(--border)" strokeDasharray="3 3" />
          <XAxis dataKey="label" stroke="var(--text-faint)" fontSize={11} />
          <YAxis stroke="var(--text-faint)" fontSize={11} />
          <Tooltip
            contentStyle={{
              background: "var(--bg-raised)",
              border: "1px solid var(--border)",
              borderRadius: 8,
            }}
            labelStyle={{ color: "var(--text)" }}
          />
          <Legend wrapperStyle={{ fontSize: 12 }} />
          <Line
            type="monotone"
            dataKey="scopeSp"
            name="Scope"
            stroke="var(--text-faint)"
            strokeDasharray="4 4"
            dot={false}
          />
          <Line
            type="monotone"
            dataKey="idealSp"
            name="Ideal"
            stroke="var(--accent)"
            strokeDasharray="2 4"
            dot={false}
          />
          <Line
            type="monotone"
            dataKey="doneSp"
            name="Done"
            stroke="var(--good)"
            strokeWidth={2}
            dot={{ r: 3 }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}

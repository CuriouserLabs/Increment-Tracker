// High-signal insight cards, hard-capped at 7 by the backend. Each card
// explains why it fired.

import type { Insight } from "@/api/types";

export function InsightsPanel({ insights }: { insights: Insight[] }) {
  if (insights.length === 0) return null;
  return (
    <details className="drawer card mb" open={insights.some((i) => i.severity === "critical")}>
      <summary>
        Insights ({insights.length})
        {insights.some((i) => i.severity === "critical") && " — needs attention"}
      </summary>
      <div className="mt">
        {insights.map((i) => (
          <div key={i.id} className={`insight ${i.severity}`}>
            <div>
              <div className="title">{i.title}</div>
              <div className="detail">{i.detail}</div>
            </div>
          </div>
        ))}
      </div>
    </details>
  );
}

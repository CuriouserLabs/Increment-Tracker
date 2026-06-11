interface Props {
  label: string;
  value: string;
  sub?: string;
  tone?: "good" | "warn" | "bad" | "neutral";
  onClick?: () => void;
}

export function KpiCard({ label, value, sub, tone = "neutral", onClick }: Props) {
  return (
    <div className={`kpi ${tone !== "neutral" ? tone : ""}`} onClick={onClick}>
      <div className="label">{label}</div>
      <div className="value">{value}</div>
      {sub && <div className="sub">{sub}</div>}
    </div>
  );
}

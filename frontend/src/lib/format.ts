// Formatting helpers — presentation only, no business math.

export function fmtSp(n: number): string {
  return Number.isInteger(n) ? n.toString() : n.toFixed(1);
}

export function fmtPct(p: number): string {
  return `${Math.round(p * 100)}%`;
}

/** "58% — 210/362 SP": always pair the percentage with its fraction. */
export function fmtProgress(done: number, total: number): string {
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;
  return `${pct}% — ${fmtSp(done)}/${fmtSp(total)} SP`;
}

export function fmtDate(iso: string | null | undefined): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function fmtDateLong(iso: string | null | undefined): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export function timeAgo(iso: string | null | undefined): string {
  if (!iso) return "never";
  const ms = Date.now() - new Date(iso).getTime();
  const min = Math.floor(ms / 60_000);
  if (min < 1) return "just now";
  if (min < 60) return `${min} min ago`;
  const h = Math.floor(min / 60);
  if (h < 24) return `${h} h ago`;
  return `${Math.floor(h / 24)} d ago`;
}

/** Fraction of the way `d` sits between `start` and `end` (clamped 0..1). */
export function timeFraction(start: string, end: string, d: string | Date): number {
  const s = new Date(start).getTime();
  const e = new Date(end).getTime();
  const t = (d instanceof Date ? d : new Date(d)).getTime();
  if (e <= s) return t >= e ? 1 : 0;
  return Math.min(1, Math.max(0, (t - s) / (e - s)));
}

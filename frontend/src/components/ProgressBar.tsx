// Story-point progress bar: solid = done, hatched = in progress.
// Binary progress model — in-flight work is visible but never counted.

import { fmtProgress } from "@/lib/format";

interface Props {
  done: number;
  inProgress?: number;
  total: number;
  showLabel?: boolean;
}

export function ProgressBar({ done, inProgress = 0, total, showLabel = true }: Props) {
  const donePct = total > 0 ? (done / total) * 100 : 0;
  const inProgPct = total > 0 ? (inProgress / total) * 100 : 0;
  return (
    <div className="pbar">
      <div className="track">
        <div className="done" style={{ width: `${donePct}%` }} />
        <div className="inprogress" style={{ width: `${inProgPct}%` }} />
      </div>
      {showLabel && <span className="label">{fmtProgress(done, total)}</span>}
    </div>
  );
}

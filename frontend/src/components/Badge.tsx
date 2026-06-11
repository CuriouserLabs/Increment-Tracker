// One consistent badge vocabulary across the whole app (spec §6).

type Kind =
  | "risk"
  | "spill"
  | "blocked"
  | "reopened"
  | "added"
  | "unestimated"
  | "carried"
  | "removed"
  | "nochildren"
  | "done";

const GLYPHS: Record<Kind, string> = {
  risk: "⚠",
  spill: "⮔",
  blocked: "⛔",
  reopened: "↩",
  added: "＋",
  unestimated: "∅",
  carried: "⤴",
  removed: "✕",
  nochildren: "▢",
  done: "✓",
};

const LABELS: Record<Kind, string> = {
  risk: "at risk",
  spill: "spilled",
  blocked: "blocked",
  reopened: "reopened",
  added: "added late",
  unestimated: "unestimated",
  carried: "carried over",
  removed: "removed from plan",
  nochildren: "no breakdown",
  done: "done",
};

interface Props {
  kind: Kind;
  count?: number;
  label?: string;
  title?: string;
}

export function Badge({ kind, count, label, title }: Props) {
  return (
    <span className={`badge ${kind}`} title={title ?? LABELS[kind]}>
      {GLYPHS[kind]}
      {label ?? LABELS[kind]}
      {count != null && count > 1 ? ` ×${count}` : ""}
    </span>
  );
}

import type { StatusCategory } from "@/api/types";

export function StatusChip({ category, label }: { category: StatusCategory; label: string }) {
  return <span className={`status-chip ${category}`}>{label}</span>;
}

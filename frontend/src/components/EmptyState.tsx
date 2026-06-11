import type { ReactNode } from "react";

interface Props {
  title: string;
  message: string;
  action?: ReactNode;
}

export function EmptyState({ title, message, action }: Props) {
  return (
    <div className="empty">
      <h4>{title}</h4>
      <p>{message}</p>
      {action}
    </div>
  );
}

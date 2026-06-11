// Jira deep links: the app never re-implements Jira's issue view — the
// deepest drill-down level is always "Open in Jira".

import { openUrl } from "@tauri-apps/plugin-opener";

export function openInJira(baseUrl: string | undefined, issueKey: string) {
  if (!baseUrl) return;
  void openUrl(`${baseUrl.replace(/\/$/, "")}/browse/${issueKey}`);
}

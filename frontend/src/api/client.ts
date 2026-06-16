// Typed wrappers around Tauri commands. The only place `invoke` is called.

import { invoke } from "@tauri-apps/api/core";
import type {
  ConnectionTestResult,
  ConnectionView,
  DashboardData,
  EpicDetail,
  EpicListRow,
  FieldMapping,
  Increment,
  IncrementInput,
  JqlValidation,
  ProjectView,
  SaveConnectionInput,
  SettingsView,
  SpilloverReport,
  SprintCompletionPoint,
  SprintNaming,
  SprintDetail,
  SyncSummary,
  TestConnectionInput,
} from "./types";

export const api = {
  // Connection
  testConnection: (input: TestConnectionInput) =>
    invoke<ConnectionTestResult>("test_connection", { input }),
  saveConnection: (input: SaveConnectionInput) =>
    invoke<ConnectionView>("save_connection", { input }),
  listProjects: () => invoke<ProjectView[]>("list_projects"),
  discoverFields: () => invoke<FieldMapping>("discover_fields"),
  saveFieldMapping: (mapping: FieldMapping) =>
    invoke<void>("save_field_mapping", { mapping }),
  saveProjects: (projects: string[]) => invoke<void>("save_projects", { projects }),

  // Settings
  getSettings: () => invoke<SettingsView>("get_settings"),
  saveBlockedStatuses: (statuses: string[]) =>
    invoke<void>("save_blocked_statuses", { statuses }),
  saveEpicChildrenClause: (clause: string | null) =>
    invoke<void>("save_epic_children_clause", { clause }),
  saveSprintNaming: (naming: SprintNaming) =>
    invoke<void>("save_sprint_naming", { naming }),
  listIncrements: () => invoke<Increment[]>("list_increments"),
  saveIncrement: (input: IncrementInput) => invoke<Increment>("save_increment", { input }),
  deleteIncrement: (id: number) => invoke<void>("delete_increment", { id }),
  setActiveIncrement: (id: number) => invoke<void>("set_active_increment", { id }),
  validateJql: (jql: string) => invoke<JqlValidation>("validate_jql", { jql }),
  clearLocalData: () => invoke<void>("clear_local_data"),

  // Sync
  syncIncrement: (incrementId: number) =>
    invoke<SyncSummary>("sync_increment", { incrementId }),

  // Queries
  getDashboard: (incrementId: number) =>
    invoke<DashboardData>("get_dashboard", { incrementId }),
  getEpics: (incrementId: number) => invoke<EpicListRow[]>("get_epics", { incrementId }),
  getEpicDetail: (incrementId: number, epicKey: string) =>
    invoke<EpicDetail>("get_epic_detail", { incrementId, epicKey }),
  getSprints: (incrementId: number) =>
    invoke<SprintCompletionPoint[]>("get_sprints", { incrementId }),
  getSprintDetail: (incrementId: number, sprintId: number) =>
    invoke<SprintDetail>("get_sprint_detail", { incrementId, sprintId }),
  getSpillover: (incrementId: number) =>
    invoke<SpilloverReport>("get_spillover", { incrementId }),
};

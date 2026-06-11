import { Navigate, Route, Routes } from "react-router-dom";

import { EpicDetailPage } from "@/features/epics/EpicDetailPage";
import { EpicsPage } from "@/features/epics/EpicsPage";
import { HomePage } from "@/features/home/HomePage";
import { SettingsPage } from "@/features/settings/SettingsPage";
import { SpilloverPage } from "@/features/spillover/SpilloverPage";
import { SprintDetailPage } from "@/features/sprints/SprintDetailPage";
import { SprintsPage } from "@/features/sprints/SprintsPage";
import { Layout } from "./Layout";

export function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<HomePage />} />
        <Route path="epics" element={<EpicsPage />} />
        <Route path="epics/:epicKey" element={<EpicDetailPage />} />
        <Route path="sprints" element={<SprintsPage />} />
        <Route path="sprints/:sprintId" element={<SprintDetailPage />} />
        <Route path="spillover" element={<SpilloverPage />} />
        <Route path="settings" element={<SettingsPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}

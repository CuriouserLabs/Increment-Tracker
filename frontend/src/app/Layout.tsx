import { useEffect } from "react";
import { Outlet, useNavigate } from "react-router-dom";

import { useSettings } from "@/api/queries";
import { useUiStore } from "@/store/ui";
import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

export function Layout() {
  const { data: settings, isLoading } = useSettings();
  const { incrementId, setIncrementId } = useUiStore();
  const navigate = useNavigate();

  // Seed the selected increment from settings on first load.
  useEffect(() => {
    if (incrementId == null && settings?.activeIncrementId != null) {
      setIncrementId(settings.activeIncrementId);
    }
  }, [incrementId, settings, setIncrementId]);

  // First-run: no connection yet -> take the user to Settings.
  useEffect(() => {
    if (!isLoading && settings && !settings.connection) {
      navigate("/settings", { replace: true });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isLoading, settings?.connection == null]);

  return (
    <div className="app-shell">
      <Sidebar />
      <div className="main">
        <TopBar />
        <main className="content">
          <Outlet />
        </main>
      </div>
    </div>
  );
}

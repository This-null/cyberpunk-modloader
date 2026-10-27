import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { LogsScreen } from "../features/logs/LogsScreen";
import { NexusScreen } from "../features/nexus/NexusScreen";
import { ModsScreen } from "../features/mods/ModsScreen";
import { SettingsScreen } from "../features/settings/SettingsScreen";
import { useLocale } from "../i18n";
import { useErrors } from "../stores/errors";
import { useMods } from "../stores/mods";
import { useNexus } from "../stores/nexus";
import { useNightcoop } from "../stores/nightcoop";
import { useUpdate } from "../stores/update";
import { applyMotion, applyTheme, useSettings } from "../stores/settings";
import { ErrorDialog } from "./ErrorDialog";
import { DownloadsBanner, IncomingDialog, UpdateBanner, useIncomingEvents } from "./Incoming";
import { Splash } from "./Splash";
import { Sidebar, type Page } from "./Sidebar";
import { TitleBar } from "./TitleBar";

export function App() {
  const [page, setPage] = useState<Page>("mods");
  const theme = useSettings((s) => s.theme);
  const motion = useSettings((s) => s.motion);
  const localeChoice = useLocale((s) => s.choice);
  const gameDir = useSettings((s) => s.gameDir);

  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  useIncomingEvents();
  const updates = useSettings((s) => s.updates);

  useEffect(() => {
    if (updates !== "on") return;
    const id = setTimeout(() => void useUpdate.getState().check(), 4000);
    return () => clearTimeout(id);
  }, [updates]);

  useEffect(() => {
    applyMotion(motion);
  }, [motion]);

  useEffect(() => {
    void useSettings
      .getState()
      .init()
      .then(() => useMods.getState().refresh())
      .then(() => useNexus.getState().load());
  }, []);

  useEffect(() => {
    void useNightcoop.getState().fetch();
  }, [localeChoice]);

  useEffect(() => {
    void useNightcoop.getState().detect();
  }, [gameDir]);

  useEffect(() => {
    const unlisten = listen<string>("fatal-error", (e) =>
      useErrors.getState().report({ code: "panic", message: e.payload }),
    );
    return () => {
      void unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="flex h-full flex-col">
      <Splash />
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar page={page} onChange={setPage} />
        <div className="flex min-w-0 flex-1 flex-col">
        <UpdateBanner />
        <DownloadsBanner />
        <main className="min-w-0 flex-1 overflow-auto p-6">
          <div key={page} className="anim-page h-full">
            {page === "mods" ? (
              <ModsScreen />
            ) : page === "nexus" ? (
              <NexusScreen onSettings={() => setPage("settings")} />
            ) : page === "logs" ? (
              <LogsScreen />
            ) : (
              <SettingsScreen />
            )}
          </div>
        </main>
        </div>
      </div>
      <IncomingDialog />
      <ErrorDialog />
    </div>
  );
}

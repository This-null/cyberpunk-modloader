import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { LogsScreen } from "../features/logs/LogsScreen";
import { NexusScreen } from "../features/nexus/NexusScreen";
import { ModsScreen } from "../features/mods/ModsScreen";
import { ProfileScreen } from "../features/profile/ProfileScreen";
import { SettingsScreen } from "../features/settings/SettingsScreen";
import { useLocale } from "../i18n";
import { useErrors } from "../stores/errors";
import { ipc } from "../lib/ipc";
import { useFrameworks } from "../stores/frameworks";
import { useMods } from "../stores/mods";
import { useNav } from "../stores/nav";
import { useModUpdates } from "../stores/modUpdates";
import { useNexus } from "../stores/nexus";
import { useNightcoop } from "../stores/nightcoop";
import { useUpdate } from "../stores/update";
import { applyMotion, applyTheme, useSettings } from "../stores/settings";
import { FomodDialog } from "../features/mods/FomodDialog";
import { ErrorDialog } from "./ErrorDialog";
import { DownloadsBanner, IncomingDialog, UpdateBanner, useIncomingEvents } from "./Incoming";
import { Splash } from "./Splash";
import { Sidebar } from "./Sidebar";
import { TitleBar } from "./TitleBar";

export function App() {
  const page = useNav((s) => s.page);
  const setPage = useNav((s) => s.setPage);
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

  const allMods = useMods((s) => s.mods);

  useEffect(() => {
    void useFrameworks.getState().load();
  }, [allMods, gameDir]);

  useEffect(() => {
    const refresh = () => void useFrameworks.getState().load();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, []);

  const connected = useNexus((s) => s.status?.connected ?? false);
  const premium = useNexus((s) => s.status?.user?.is_premium ?? false);
  const modUpdates = useSettings((s) => s.modUpdates);
  const nexusItems = useNexus((s) => s.items);
  const updating = useModUpdates((s) => s.running);

  useEffect(() => {
    if (!connected) return;
    void ipc
      .nexusFillCovers()
      .then((n) => {
        if (n > 0) {
          useMods.setState({ images: {} });
          return useMods.getState().refresh();
        }
      })
      .catch(() => {});
  }, [connected]);

  const linked = useMods((s) => s.mods.filter((m) => m.nexus_id).map((m) => `${m.name}:${m.nexus_id}`).join("|"));
  const seenLinks = useRef(linked);

  useEffect(() => {
    if (seenLinks.current === linked) return;
    seenLinks.current = linked;
    if (!connected) return;
    useMods.setState({ images: {} });
    void useNexus.getState().check();
  }, [linked, connected]);

  useEffect(() => {
    if (modUpdates === "auto" && premium && !updating) void useModUpdates.getState().auto();
  }, [modUpdates, premium, nexusItems, updating]);

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
            ) : page === "profile" ? (
              <ProfileScreen />
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
      <FomodDialog />
      <ErrorDialog />
    </div>
  );
}

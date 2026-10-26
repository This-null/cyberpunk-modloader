import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./app/App";
import { startPrefsSync } from "./lib/prefs";
import { applyMotion, applyTheme, useSettings } from "./stores/settings";
import "./index.css";

async function boot() {
  if (import.meta.env.DEV && new URLSearchParams(window.location.search).has("mock")) {
    const { installMock } = await import("./dev/mock");
    installMock();
  }
  await startPrefsSync().catch(() => {});
  const { theme, motion } = useSettings.getState();
  applyTheme(theme);
  applyMotion(motion);
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void boot();

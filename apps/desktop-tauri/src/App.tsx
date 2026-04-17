import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { checkForUpdates, getBootstrapState, setSurfaceMode } from "./lib/tauri";
import { useSurfaceSnapshot } from "./hooks/useSurfaceSnapshot";
import Settings from "./surfaces/Settings";
import TrayPanel from "./surfaces/TrayPanel";
import PopOutPanel from "./surfaces/PopOutPanel";
import { LocaleProvider, useLocale } from "./i18n/LocaleProvider";
import type { BootstrapState } from "./types/bridge";
import type { SurfaceSnapshot } from "./hooks/useSurfaceSnapshot";

export default function App() {
  return (
    <LocaleProvider>
      <AppInner />
    </LocaleProvider>
  );
}

function AppInner() {
  const surface = useSurfaceSnapshot();
  const [state, setState] = useState<BootstrapState | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    getBootstrapState()
      .then((bootstrap) => {
        if (!cancelled) {
          setState(bootstrap);
        }
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      });

    // Fire-and-forget initial update check; results flow via events.
    checkForUpdates().catch(() => {});

    // Listen for user-registered global shortcut events from the
    // `register_global_shortcut` command. The persistent shortcut (bound via
    // shortcut_bridge::plugin) already toggles the tray panel natively, so
    // this listener is a no-op fallback for ad-hoc capture-mode registrations.
    const unlistenPromise = listen<string>("global-shortcut-triggered", () => {
      void setSurfaceMode("trayPanel", { kind: "summary" }).catch(() => {});
    });

    return () => {
      cancelled = true;
      void unlistenPromise.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  if (error) {
    return (
      <main className="shell">
        <section className="panel error">
          <h2>Bootstrap failed</h2>
          <p>{error}</p>
        </section>
      </main>
    );
  }

  if (!state) {
    return (
      <main className="shell">
        <section className="panel">
          <h2>Loading shell contract…</h2>
          <p>Waiting for the Rust bridge to describe providers, surfaces, and settings.</p>
        </section>
      </main>
    );
  }

  return <SurfaceRouter surface={surface} state={state} />;
}

function SurfaceRouter({
  surface,
  state,
}: {
  surface: SurfaceSnapshot;
  state: BootstrapState;
}) {
  switch (surface.mode) {
    case "hidden":
      return null;
    case "trayPanel":
      return <TrayPanel state={state} />;
    case "popOut": {
      const providerId =
        surface.target.kind === "provider"
          ? surface.target.providerId
          : undefined;
      return <PopOutPanel state={state} providerId={providerId} />;
    }
    case "settings":
      return <SettingsLayout state={state} />;
    default:
      return <TrayPanel state={state} />;
  }
}

function SettingsLayout({ state }: { state: BootstrapState }) {
  const { t } = useLocale();
  return (
    <main className="shell">
      <header className="hero">
        <p className="eyebrow">{t("ActionSettings")}</p>
        <h1>{t("ActionSettings")}</h1>
      </header>
      <Settings state={state} />
    </main>
  );
}

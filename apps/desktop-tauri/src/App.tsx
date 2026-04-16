import { useEffect, useState } from "react";
import { checkForUpdates, getBootstrapState, getProofConfig } from "./lib/tauri";
import { useSurfaceMode } from "./hooks/useSurfaceMode";
import Settings from "./surfaces/Settings";
import TrayPanel from "./surfaces/TrayPanel";
import PopOutPanel from "./surfaces/PopOutPanel";
import type { BootstrapState, ProofConfig, SurfaceMode, SurfaceTarget } from "./types/bridge";

export default function App() {
  const surface = useSurfaceMode();
  const [state, setState] = useState<BootstrapState | null>(null);
  const [proofConfig, setProofConfig] = useState<ProofConfig | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    Promise.all([getBootstrapState(), getProofConfig()])
      .then(([bootstrap, proof]) => {
        if (!cancelled) {
          setState(bootstrap);
          setProofConfig(proof);
        }
      })
      .catch((cause: unknown) => {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      });

    // Fire-and-forget initial update check; results flow via events.
    checkForUpdates().catch(() => {});

    return () => {
      cancelled = true;
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

  return (
    <SurfaceRouter
      mode={surface.mode}
      target={surface.target}
      state={state}
      proofConfig={proofConfig}
    />
  );
}

function SurfaceRouter({
  mode,
  target,
  state,
  proofConfig,
}: {
  mode: SurfaceMode;
  target: SurfaceTarget;
  state: BootstrapState;
  proofConfig: ProofConfig | null;
}) {
  switch (mode) {
    case "hidden":
      return null;
    case "trayPanel":
      return <TrayPanel state={state} target={target} />;
    case "popOut":
      return <PopOutPanel state={state} target={target} />;
    case "settings":
      return (
        <SettingsLayout
          state={state}
          initialTab={
            target.kind === "settings"
              ? target.tab
              : proofConfig?.settingsTab ?? undefined
          }
        />
      );
    default:
      return <TrayPanel state={state} target={target} />;
  }
}

function SettingsLayout({ state, initialTab }: { state: BootstrapState; initialTab?: string }) {
  return (
    <main className="shell">
      <header className="hero">
        <p className="eyebrow">Settings</p>
        <h1>Preferences</h1>
      </header>
      <Settings state={state} initialTab={initialTab} />
    </main>
  );
}

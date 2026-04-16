import { useCallback, useEffect, useState } from "react";
import type {
  ApiKeyInfoBridge,
  ApiKeyProviderInfoBridge,
  AppInfoBridge,
  BootstrapState,
  CookieInfoBridge,
  ProviderCatalogEntry,
  SettingsUpdate,
} from "../types/bridge";
import { useSettings } from "../hooks/useSettings";
import { useSurfaceTarget } from "../hooks/useSurfaceMode";
import { useUpdateState } from "../hooks/useUpdateState";
import {
  getApiKeyProviders,
  getApiKeys,
  getAppInfo,
  getManualCookies,
  removeApiKey,
  removeManualCookie,
  setApiKey,
  setManualCookie,
} from "../lib/tauri";

// ── tiny reusable controls ──────────────────────────────────────────

function Toggle({
  checked,
  onChange,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      className={`toggle ${checked ? "toggle--on" : ""}`}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    />
  );
}

function Select({
  value,
  options,
  onChange,
  disabled,
}: {
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <select
      className="select"
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

function NumberInput({
  value,
  min,
  max,
  step,
  onChange,
  disabled,
}: {
  value: number;
  min?: number;
  max?: number;
  step?: number;
  onChange: (v: number) => void;
  disabled?: boolean;
}) {
  return (
    <input
      type="number"
      className="number-input"
      value={value}
      min={min}
      max={max}
      step={step}
      disabled={disabled}
      onChange={(e) => {
        const n = Number(e.target.value);
        if (!Number.isNaN(n)) onChange(n);
      }}
    />
  );
}

function TextInput({
  value,
  placeholder,
  onChange,
  disabled,
}: {
  value: string;
  placeholder?: string;
  onChange: (v: string) => void;
  disabled?: boolean;
}) {
  return (
    <input
      type="text"
      className="text-input"
      value={value}
      placeholder={placeholder}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}

// ── field row ────────────────────────────────────────────────────────

function Field({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-field">
      <div className="settings-field__text">
        <span className="settings-field__label">{label}</span>
        {description && (
          <span className="settings-field__desc">{description}</span>
        )}
      </div>
      <div className="settings-field__control">{children}</div>
    </div>
  );
}

// ── tab types ────────────────────────────────────────────────────────

type SettingsTab =
  | "general"
  | "providers"
  | "display"
  | "apiKeys"
  | "cookies"
  | "advanced"
  | "about";

const TAB_META: { id: SettingsTab; label: string; icon: string }[] = [
  { id: "general", label: "General", icon: "⚙" },
  { id: "providers", label: "Providers", icon: "◉" },
  { id: "display", label: "Display", icon: "◧" },
  { id: "apiKeys", label: "API Keys", icon: "🔑" },
  { id: "cookies", label: "Cookies", icon: "🍪" },
  { id: "advanced", label: "Advanced", icon: "⌘" },
  { id: "about", label: "About", icon: "ℹ" },
];

// ── main component ──────────────────────────────────────────────────

function isSettingsTab(value: string): value is SettingsTab {
  return TAB_META.some((t) => t.id === value);
}

export default function Settings({ state, initialTab }: { state: BootstrapState; initialTab?: string }) {
  const { settings, saving, error, update } = useSettings(state.settings);
  const resolvedInitial: SettingsTab =
    initialTab && isSettingsTab(initialTab) ? initialTab : "general";
  const [activeTab, setActiveTab] = useState<SettingsTab>(resolvedInitial);
  const shellTarget = useSurfaceTarget("settings");

  useEffect(() => {
    if (shellTarget?.kind !== "settings" || !isSettingsTab(shellTarget.tab)) {
      return;
    }

    const nextTab: SettingsTab = shellTarget.tab;
    setActiveTab((current) =>
      current === nextTab ? current : nextTab,
    );
  }, [shellTarget]);

  const set = (patch: SettingsUpdate) => void update(patch);

  return (
    <div className="settings">
      {/* tab bar */}
      <nav className="settings-tabs" role="tablist">
        {TAB_META.map((t) => (
          <button
            key={t.id}
            role="tab"
            aria-selected={activeTab === t.id}
            className={`settings-tab ${activeTab === t.id ? "settings-tab--active" : ""}`}
            onClick={() => setActiveTab(t.id)}
          >
            <span className="settings-tab__icon">{t.icon}</span>
            {t.label}
          </button>
        ))}
      </nav>

      {/* status bar */}
      {(saving || error) && (
        <div className={`settings-status ${error ? "settings-status--error" : ""}`}>
          {saving ? "Saving…" : error}
        </div>
      )}

      {/* tab panels */}
      <div className="settings-body">
        {activeTab === "general" && (
          <GeneralTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "providers" && (
          <ProvidersTab
            settings={settings}
            providers={state.providers}
            set={set}
            saving={saving}
          />
        )}
        {activeTab === "display" && (
          <DisplayTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "advanced" && (
          <AdvancedTab settings={settings} set={set} saving={saving} />
        )}
        {activeTab === "apiKeys" && (
          <ApiKeysTab providers={state.providers} />
        )}
        {activeTab === "cookies" && (
          <CookiesTab providers={state.providers} />
        )}
        {activeTab === "about" && <AboutTab />}
      </div>
    </div>
  );
}

// ── General ──────────────────────────────────────────────────────────

interface TabProps {
  settings: BootstrapState["settings"];
  set: (p: SettingsUpdate) => void;
  saving: boolean;
}

function GeneralTab({ settings, set, saving }: TabProps) {
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Startup</h3>
      <Field label="Start at login" description="Launch CodexBar automatically when you sign in.">
        <Toggle
          checked={settings.startAtLogin}
          disabled={saving}
          onChange={(v) => set({ startAtLogin: v })}
        />
      </Field>

      <h3 className="settings-section__title">Refresh</h3>
      <Field label="Refresh interval" description="Seconds between automatic provider refreshes (0 = manual).">
        <NumberInput
          value={settings.refreshIntervalSecs}
          min={0}
          max={3600}
          step={30}
          disabled={saving}
          onChange={(v) => set({ refreshIntervalSecs: v })}
        />
      </Field>

      <h3 className="settings-section__title">Notifications</h3>
      <Field label="Show notifications" description="Display desktop alerts for usage thresholds.">
        <Toggle
          checked={settings.showNotifications}
          disabled={saving}
          onChange={(v) => set({ showNotifications: v })}
        />
      </Field>

      <h3 className="settings-section__title">Keyboard</h3>
      <Field label="Global shortcut" description="Key combination to toggle the tray panel.">
        <TextInput
          value={settings.globalShortcut}
          placeholder="Ctrl+Shift+U"
          disabled={saving}
          onChange={(v) => set({ globalShortcut: v })}
        />
      </Field>
    </section>
  );
}

// ── Providers ────────────────────────────────────────────────────────

function ProvidersTab({
  settings,
  providers,
  set,
  saving,
}: TabProps & { providers: ProviderCatalogEntry[] }) {
  const enabled = new Set(settings.enabledProviders);

  const toggle = (id: string, on: boolean) => {
    const next = new Set(enabled);
    if (on) next.add(id);
    else next.delete(id);
    set({ enabledProviders: [...next].sort() });
  };

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Enabled providers</h3>
      <p className="settings-section__hint">
        Toggle which providers are visible in the tray and panels.
      </p>
      <ul className="provider-list">
        {providers.map((p) => (
          <li key={p.id} className="provider-row">
            <div className="provider-row__info">
              <strong>{p.displayName}</strong>
              <span className="provider-row__meta">
                {p.id}
                {p.cookieDomain ? ` · ${p.cookieDomain}` : " · token-based"}
              </span>
            </div>
            <Toggle
              checked={enabled.has(p.id)}
              disabled={saving}
              onChange={(v) => toggle(p.id, v)}
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

// ── Display ──────────────────────────────────────────────────────────

function DisplayTab({ settings, set, saving }: TabProps) {
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Tray icon</h3>
      <Field label="Tray icon mode" description="Single unified icon or one icon per enabled provider.">
        <Select
          value={settings.trayIconMode}
          disabled={saving}
          options={[
            { value: "single", label: "Single" },
            { value: "perProvider", label: "Per provider" },
          ]}
          onChange={(v) => set({ trayIconMode: v })}
        />
      </Field>

      <Field label="Display mode" description="Level of detail shown in the menu bar label.">
        <Select
          value={settings.menuBarDisplayMode}
          disabled={saving}
          options={[
            { value: "detailed", label: "Detailed" },
            { value: "compact", label: "Compact" },
            { value: "minimal", label: "Minimal" },
          ]}
          onChange={(v) => set({ menuBarDisplayMode: v })}
        />
      </Field>

      <h3 className="settings-section__title">Usage rendering</h3>
      <Field label="Show as used" description="Display usage bars as consumed rather than remaining.">
        <Toggle
          checked={settings.showAsUsed}
          disabled={saving}
          onChange={(v) => set({ showAsUsed: v })}
        />
      </Field>

      <h3 className="settings-section__title">Animations</h3>
      <Field label="Enable animations" description="Smooth transitions and animated progress bars.">
        <Toggle
          checked={settings.enableAnimations}
          disabled={saving}
          onChange={(v) => set({ enableAnimations: v })}
        />
      </Field>
      <Field label="Surprise animations" description="Fun confetti and particle effects at milestones.">
        <Toggle
          checked={settings.surpriseAnimations}
          disabled={saving}
          onChange={(v) => set({ surpriseAnimations: v })}
        />
      </Field>

      <h3 className="settings-section__title">Privacy</h3>
      <Field label="Hide personal info" description="Mask emails and account names in the UI.">
        <Toggle
          checked={settings.hidePersonalInfo}
          disabled={saving}
          onChange={(v) => set({ hidePersonalInfo: v })}
        />
      </Field>
    </section>
  );
}

// ── Advanced ─────────────────────────────────────────────────────────

function AdvancedTab({ settings, set, saving }: TabProps) {
  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Updates</h3>
      <Field label="Update channel" description="Stable for production releases, Beta for early access.">
        <Select
          value={settings.updateChannel}
          disabled={saving}
          options={[
            { value: "stable", label: "Stable" },
            { value: "beta", label: "Beta" },
          ]}
          onChange={(v) => set({ updateChannel: v })}
        />
      </Field>

      <h3 className="settings-section__title">Language</h3>
      <Field label="Interface language" description="Language used throughout the UI.">
        <Select
          value={settings.uiLanguage}
          disabled={saving}
          options={[
            { value: "english", label: "English" },
            { value: "chinese", label: "中文" },
          ]}
          onChange={(v) => set({ uiLanguage: v })}
        />
      </Field>

      <h3 className="settings-section__title">Time</h3>
      <Field label="Reset time relative" description="Show reset countdowns as relative times (e.g. 'in 3h').">
        <Toggle
          checked={settings.resetTimeRelative}
          disabled={saving}
          onChange={(v) => set({ resetTimeRelative: v })}
        />
      </Field>
    </section>
  );
}

// ── API Keys ─────────────────────────────────────────────────────────

function ApiKeysTab({ providers }: { providers: ProviderCatalogEntry[] }) {
  const [keys, setKeys] = useState<ApiKeyInfoBridge[]>([]);
  const [apiKeyProviders, setApiKeyProviders] = useState<
    ApiKeyProviderInfoBridge[]
  >([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Which provider is currently being edited
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [editLabel, setEditLabel] = useState("");

  const reload = useCallback(async () => {
    try {
      const [k, p] = await Promise.all([getApiKeys(), getApiKeyProviders()]);
      setKeys(k);
      setApiKeyProviders(p);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const handleSave = async (providerId: string) => {
    if (!editValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const next = await setApiKey(
        providerId,
        editValue.trim(),
        editLabel.trim() || undefined,
      );
      setKeys(next);
      setEditingId(null);
      setEditValue("");
      setEditLabel("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (providerId: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await removeApiKey(providerId);
      setKeys(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  // Build a lookup of provider display names
  const providerNames = new Map(providers.map((p) => [p.id, p.displayName]));

  // Merge: show api-key providers with their saved state
  const keyMap = new Map(keys.map((k) => [k.providerId, k]));

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">API Keys</h3>
      <p className="settings-section__hint">
        Configure API keys for providers that use token-based authentication.
        Keys are stored locally and never transmitted.
      </p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      <ul className="credential-list">
        {apiKeyProviders.map((p) => {
          const saved = keyMap.get(p.id);
          const isEditing = editingId === p.id;
          const displayName = providerNames.get(p.id) ?? p.displayName;

          return (
            <li key={p.id} className="credential-card">
              <div className="credential-card__header">
                <div className="credential-card__info">
                  <strong>{displayName}</strong>
                  <span className="credential-card__meta">
                    {saved ? (
                      <>
                        <span className="credential-card__badge credential-card__badge--set">
                          Configured
                        </span>
                        <span className="credential-card__masked">
                          {saved.maskedKey}
                        </span>
                        {saved.label && (
                          <span className="credential-card__label">
                            {saved.label}
                          </span>
                        )}
                        <span className="credential-card__date">
                          Saved {saved.savedAt}
                        </span>
                      </>
                    ) : (
                      <span className="credential-card__badge credential-card__badge--unset">
                        Not set
                      </span>
                    )}
                  </span>
                </div>
                <div className="credential-card__actions">
                  {!isEditing && (
                    <button
                      className="credential-btn"
                      disabled={busy}
                      onClick={() => {
                        setEditingId(p.id);
                        setEditValue("");
                        setEditLabel(saved?.label ?? "");
                      }}
                    >
                      {saved ? "Update" : "Add Key"}
                    </button>
                  )}
                  {saved && !isEditing && (
                    <button
                      className="credential-btn credential-btn--danger"
                      disabled={busy}
                      onClick={() => void handleRemove(p.id)}
                    >
                      Remove
                    </button>
                  )}
                </div>
              </div>

              {p.help && !isEditing && (
                <p className="credential-card__help">{p.help}</p>
              )}

              {p.dashboardUrl && !isEditing && (
                <a
                  className="credential-card__link"
                  href={p.dashboardUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Open dashboard ↗
                </a>
              )}

              {isEditing && (
                <div className="credential-card__edit">
                  <input
                    type="password"
                    className="text-input credential-card__input"
                    placeholder="Paste API key…"
                    autoComplete="off"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    disabled={busy}
                  />
                  <input
                    type="text"
                    className="text-input credential-card__input credential-card__input--label"
                    placeholder="Label (optional)"
                    value={editLabel}
                    onChange={(e) => setEditLabel(e.target.value)}
                    disabled={busy}
                  />
                  <div className="credential-card__edit-actions">
                    <button
                      className="credential-btn credential-btn--primary"
                      disabled={busy || !editValue.trim()}
                      onClick={() => void handleSave(p.id)}
                    >
                      Save
                    </button>
                    <button
                      className="credential-btn"
                      disabled={busy}
                      onClick={() => {
                        setEditingId(null);
                        setEditValue("");
                        setEditLabel("");
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

// ── Cookies ──────────────────────────────────────────────────────────

function CookiesTab({ providers }: { providers: ProviderCatalogEntry[] }) {
  const [cookies, setCookies] = useState<CookieInfoBridge[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Add-cookie form state
  const [addProviderId, setAddProviderId] = useState("");
  const [addCookieValue, setAddCookieValue] = useState("");

  const reload = useCallback(async () => {
    try {
      setCookies(await getManualCookies());
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  // Only show providers with a cookie domain
  const cookieProviders = providers.filter((p) => p.cookieDomain !== null);

  const handleAdd = async () => {
    if (!addProviderId || !addCookieValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const next = await setManualCookie(addProviderId, addCookieValue.trim());
      setCookies(next);
      setAddProviderId("");
      setAddCookieValue("");
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleRemove = async (providerId: string) => {
    setBusy(true);
    setError(null);
    try {
      const next = await removeManualCookie(providerId);
      setCookies(next);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="settings-section">
      <h3 className="settings-section__title">Saved Cookies</h3>
      <p className="settings-section__hint">
        Manual cookie overrides for browser-authenticated providers. These are
        used when automatic browser cookie extraction is unavailable.
      </p>

      {error && (
        <div className="settings-status settings-status--error">{error}</div>
      )}

      {cookies.length > 0 ? (
        <ul className="credential-list">
          {cookies.map((c) => (
            <li key={c.providerId} className="credential-card">
              <div className="credential-card__header">
                <div className="credential-card__info">
                  <strong>{c.provider}</strong>
                  <span className="credential-card__meta">
                    <span className="credential-card__badge credential-card__badge--set">
                      Saved
                    </span>
                    <span className="credential-card__date">
                      {c.savedAt}
                    </span>
                  </span>
                </div>
                <div className="credential-card__actions">
                  <button
                    className="credential-btn credential-btn--danger"
                    disabled={busy}
                    onClick={() => void handleRemove(c.providerId)}
                  >
                    Remove
                  </button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <p className="credential-empty">No manual cookies saved.</p>
      )}

      <h3 className="settings-section__title">Add Cookie</h3>
      <div className="credential-add-form">
        <Select
          value={addProviderId}
          options={[
            { value: "", label: "Select provider…" },
            ...cookieProviders.map((p) => ({
              value: p.id,
              label: p.displayName,
            })),
          ]}
          onChange={setAddProviderId}
          disabled={busy}
        />
        <textarea
          className="text-input credential-textarea"
          placeholder="Paste cookie header value…"
          rows={3}
          value={addCookieValue}
          onChange={(e) => setAddCookieValue(e.target.value)}
          disabled={busy}
        />
        <button
          className="credential-btn credential-btn--primary"
          disabled={busy || !addProviderId || !addCookieValue.trim()}
          onClick={() => void handleAdd()}
        >
          Save Cookie
        </button>
      </div>
    </section>
  );
}

// ── About ────────────────────────────────────────────────────────────

function AboutTab() {
  const [appInfo, setAppInfo] = useState<AppInfoBridge | null>(null);
  const { updateState, checkNow, download, apply, openRelease } =
    useUpdateState();
  const [hasChecked, setHasChecked] = useState(false);

  useEffect(() => {
    void getAppInfo().then(setAppInfo);
  }, []);

  const handleCheck = () => {
    setHasChecked(true);
    checkNow();
  };

  if (!appInfo) {
    return (
      <section className="settings-section">
        <p className="settings-section__hint">Loading…</p>
      </section>
    );
  }

  const isBusy =
    updateState.status === "checking" ||
    updateState.status === "downloading";

  return (
    <section className="settings-section about-section">
      <div className="about-header">
        <div className="about-icon">⬡</div>
        <div className="about-title-block">
          <h2 className="about-title">{appInfo.name}</h2>
          <p className="about-version">
            v{appInfo.version}
            {appInfo.buildNumber !== "dev" && (
              <span className="about-build"> · Build {appInfo.buildNumber}</span>
            )}
          </p>
        </div>
      </div>

      <p className="about-tagline">{appInfo.tagline}</p>

      <div className="about-details">
        <div className="about-detail-row">
          <span className="about-detail-label">Update channel</span>
          <span className="about-detail-value">{appInfo.updateChannel}</span>
        </div>
      </div>

      <div className="about-actions">
        <button
          className="credential-btn credential-btn--primary"
          disabled={isBusy}
          onClick={handleCheck}
        >
          {updateState.status === "checking"
            ? "Checking…"
            : "Check for Updates"}
        </button>

        {updateState.status === "available" && (
          <div className="about-update-row">
            <span className="about-update-msg">
              Update {updateState.version} available
            </span>
            {updateState.canDownload ? (
              <button
                className="credential-btn credential-btn--primary"
                onClick={download}
              >
                Download
              </button>
            ) : (
              <button className="credential-btn" onClick={openRelease}>
                View Release
              </button>
            )}
          </div>
        )}

        {updateState.status === "downloading" && (
          <span className="about-update-msg">
            Downloading…
            {updateState.progress != null &&
              ` ${Math.round(updateState.progress * 100)}%`}
          </span>
        )}

        {updateState.status === "ready" && (
          <div className="about-update-row">
            <span className="about-update-msg">Update ready to install</span>
            {updateState.canApply ? (
              <button
                className="credential-btn credential-btn--primary"
                onClick={apply}
              >
                Install &amp; Restart
              </button>
            ) : (
              <button className="credential-btn" onClick={openRelease}>
                View Release
              </button>
            )}
          </div>
        )}

        {updateState.status === "error" && (
          <span className="about-update-msg">
            Error: {updateState.error}
          </span>
        )}

        {updateState.status === "idle" && hasChecked && (
          <span className="about-update-msg">You&apos;re up to date!</span>
        )}
      </div>

      <div className="about-links">
        <a
          className="about-link"
          href="https://github.com/NessZerra/Win-CodexBar"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub
        </a>
        <span className="about-link-sep">·</span>
        <a
          className="about-link"
          href="https://codexbar.app"
          target="_blank"
          rel="noopener noreferrer"
        >
          Website
        </a>
      </div>

      <p className="about-copyright">
        NessZerra — Windows Version. MIT License.
      </p>
    </section>
  );
}

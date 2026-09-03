import { useEffect, useState } from "react"
import type { ReactNode } from "react"
import type { ProviderId } from "../state/usage"

const tabs = ["Appearance", "Monitoring", "Services", "About"] as const
const fonts = [
  "System Default",
  "Antonio",
  "DIN Condensed",
  "Alimama FangYuanTi VF",
  "Fira Code",
  "Leigo",
  "Menlo",
  "Alimama DaoLiTi",
]

type SettingsWindowProps = {
  displayFont: string
  onDisplayFontChange: (font: string) => void
  edge?: "left" | "right"
  onEdgeChange?: (edge: "left" | "right") => void
  detailAutoHideSeconds?: number
  onDetailAutoHideSecondsChange?: (seconds: number) => void
  requestedTab?: (typeof tabs)[number]
  updateState?: UpdateState
  onCheckForUpdates?: () => void
  onInstallUpdate?: () => void
  serviceStatuses?: ServiceAccountStatus[]
  onCheckServiceStatus?: (providerId: ProviderId) => void
  onBeginServiceSignIn?: (providerId: "claude" | "codex") => void
  onReplaceDeepSeekKey?: () => Promise<boolean> | boolean | void
  serviceMessage?: string | null
  cliSettings?: Record<"claude" | "codex", ProviderCliSettings>
  onCliSettingsChange?: (providerId: "claude" | "codex", value: ProviderCliSettings) => void
  wslDistributions?: string[]
  refreshIntervalSeconds?: number
  onRefreshIntervalSecondsChange?: (seconds: number) => void
  deepseekBalanceBaselineCents?: number
  onDeepSeekBalanceBaselineCentsChange?: (cents: number) => void
  notificationsEnabled?: boolean
  onNotificationsEnabledChange?: (enabled: boolean) => void
  launchAtLogin?: boolean
  onLaunchAtLoginChange?: (enabled: boolean) => void
}

export type ProviderCliSettings = {
  mode: "auto" | "nativeWindows" | "wsl"
  customPath: string | null
  wslDistribution: string | null
}

const defaultCliSettings: ProviderCliSettings = {
  mode: "auto",
  customPath: null,
  wslDistribution: null,
}

export type ServiceAccountStatus = {
  providerId: ProviderId
  connectionState: "connected" | "signInRequired" | "notInstalled" | "checking" | "unavailable"
  accountLabel?: string | null
  accountDetail?: string | null
  runtimeSource?: string | null
  cliVersion?: string | null
  checkedAt?: string | null
}

export type UpdateState = {
  phase: "idle" | "checking" | "upToDate" | "available" | "downloading" | "installing" | "failed"
  currentVersion: string
  availableVersion?: string | null
  progressPercent?: number | null
  message?: string | null
}

const defaultUpdateState: UpdateState = {
  phase: "idle",
  currentVersion: "0.2.2",
}

export function SettingsWindow({
  displayFont,
  onDisplayFontChange,
  edge = "right",
  onEdgeChange = () => {},
  detailAutoHideSeconds = 8,
  onDetailAutoHideSecondsChange = () => {},
  requestedTab,
  updateState = defaultUpdateState,
  onCheckForUpdates = () => {},
  onInstallUpdate = () => {},
  serviceStatuses = [],
  onCheckServiceStatus = () => {},
  onBeginServiceSignIn = () => {},
  onReplaceDeepSeekKey = () => {},
  serviceMessage,
  cliSettings = { claude: defaultCliSettings, codex: defaultCliSettings },
  onCliSettingsChange = () => {},
  wslDistributions = [],
  refreshIntervalSeconds = 300,
  onRefreshIntervalSecondsChange = () => {},
  deepseekBalanceBaselineCents = 10_000,
  onDeepSeekBalanceBaselineCentsChange = () => {},
  notificationsEnabled = false,
  onNotificationsEnabledChange = () => {},
  launchAtLogin = false,
  onLaunchAtLoginChange = () => {},
}: SettingsWindowProps) {
  const [activeTab, setActiveTab] = useState<(typeof tabs)[number]>("Appearance")
  useEffect(() => {
    if (requestedTab) setActiveTab(requestedTab)
  }, [requestedTab])
  return (
    <section aria-label="AI Token Meter Settings" className="settings-window settings-window--system-font" role="dialog">
      <header><div><strong>AI Token Meter</strong><small>Private AI usage, at a glance.</small></div></header>
      <nav aria-label="Settings categories" role="tablist">
        {tabs.map((tab) => (
          <button
            aria-selected={activeTab === tab}
            key={tab}
            onClick={() => setActiveTab(tab)}
            role="tab"
            type="button"
          >{tab}</button>
        ))}
      </nav>
      <div className="settings-content">
        {activeTab === "Appearance" ? (
          <>
            <SettingRow label="Display font" hint="Applies to the meter, menu and detail panels. Settings always uses the system font.">
              <select aria-label="Display font" onChange={(event) => onDisplayFontChange(event.target.value)} value={displayFont}>
                {fonts.map((font) => <option key={font}>{font}</option>)}
              </select>
              <button onClick={() => onDisplayFontChange("System Default")} type="button">Restore default font</button>
            </SettingRow>
            <SettingRow label="Screen edge" hint="The meter follows the selected display and stays outside the taskbar.">
              <select
                aria-label="Screen edge"
                onChange={(event) => onEdgeChange(event.target.value as "left" | "right")}
                value={edge}
              ><option value="right">Right</option><option value="left">Left</option></select>
            </SettingRow>
          </>
        ) : null}
        {activeTab === "Monitoring" ? (
          <>
            <SettingRow label="Refresh interval" hint="Scheduled refreshes never overlap; a manual refresh replaces an older task.">
              <DraftNumberInput
                ariaLabel="Refresh interval seconds"
                max={86_400}
                min={30}
                onCommit={onRefreshIntervalSecondsChange}
                value={refreshIntervalSeconds}
              /> seconds
            </SettingRow>
            <SettingRow label="DeepSeek balance baseline" hint="The DeepSeek ring shows the amount consumed from this reference balance.">
              ¥ <DraftNumberInput
                ariaLabel="DeepSeek balance baseline"
                format={(value) => String(value / 100)}
                max={1_000_000}
                min={1}
                onCommit={(value) => onDeepSeekBalanceBaselineCentsChange(Math.round(value * 100))}
                step={0.01}
                value={deepseekBalanceBaselineCents}
              />
            </SettingRow>
            <SettingRow label="Usage alerts" hint="Notify once at 70% and again at 90%; dropping below 10% re-arms the alerts.">
              <input
                aria-label="Usage alerts at 70% and 90%"
                checked={notificationsEnabled}
                onChange={(event) => onNotificationsEnabledChange(event.target.checked)}
                type="checkbox"
              />
            </SettingRow>
            <SettingRow label="Launch at login" hint="Start the meter after you sign in to Windows.">
              <input
                aria-label="Open AI Token Meter at login"
                checked={launchAtLogin}
                onChange={(event) => onLaunchAtLoginChange(event.target.checked)}
                type="checkbox"
              />
            </SettingRow>
            <SettingRow label="Detail auto-hide" hint="Interaction pauses the countdown.">
              <DraftNumberInput
                ariaLabel="Detail auto-hide seconds"
                max={300}
                min={1}
                onCommit={onDetailAutoHideSecondsChange}
                value={detailAutoHideSeconds}
              /> seconds
            </SettingRow>
          </>
        ) : null}
        {activeTab === "Services" ? (
          <div className="service-list">
            {(["claude", "codex"] as const).map((providerId) => {
              const name = providerId === "claude" ? "Claude Code" : "OpenAI Codex"
              const status = statusFor(serviceStatuses, providerId)
              return (
                <Service key={providerId} name={name} status={status}>
                  <CliRuntimeControls
                    name={name}
                    onChange={(value) => onCliSettingsChange(providerId, value)}
                    value={cliSettings[providerId]}
                    wslDistributions={wslDistributions}
                  />
                  <button
                    aria-label={`${status.connectionState === "connected" ? "Sign in again to" : "Sign in to"} ${name}`}
                    disabled={status.connectionState === "notInstalled" || status.connectionState === "checking"}
                    onClick={() => onBeginServiceSignIn(providerId)}
                    type="button"
                  >{status.connectionState === "connected" ? "Sign in again" : "Sign in"}</button>
                  <button
                    aria-label={`Check ${name} status`}
                    disabled={status.connectionState === "checking"}
                    onClick={() => onCheckServiceStatus(providerId)}
                    type="button"
                  >Check Status</button>
                </Service>
              )
            })}
            <Service name="DeepSeek" status={statusFor(serviceStatuses, "deepseek")}>
              <small>Windows opens a protected credential prompt; the Key never enters this WebView.</small>
              <button
                aria-label="Replace DeepSeek API Key"
                onClick={async () => {
                  await onReplaceDeepSeekKey()
                }}
                type="button"
              >Replace API Key</button>
              <button
                aria-label="Check DeepSeek status"
                onClick={() => onCheckServiceStatus("deepseek")}
                type="button"
              >Check Status</button>
            </Service>
            {serviceMessage ? <p aria-live="polite" className="service-message">{serviceMessage}</p> : null}
          </div>
        ) : null}
        {activeTab === "About" ? (
          <div className="about-card">
            <strong>AI Token Meter</strong>
            <p>Version {updateState.currentVersion}</p>
            <p>Author · Miller</p>
            <p aria-live="polite" className="update-status">{updateMessage(updateState)}</p>
            <div className="update-actions">
              <button
                disabled={["checking", "downloading", "installing"].includes(updateState.phase)}
                onClick={onCheckForUpdates}
                type="button"
              >{updateState.phase === "checking" ? "Checking…" : "Check for Updates"}</button>
              <button
                disabled={updateState.phase !== "available"}
                onClick={onInstallUpdate}
                type="button"
              >{updateState.phase === "downloading" || updateState.phase === "installing" ? "Installing…" : "Update Now"}</button>
            </div>
          </div>
        ) : null}
      </div>
    </section>
  )
}

function DraftNumberInput({
  ariaLabel,
  format = String,
  max,
  min,
  onCommit,
  step = 1,
  value,
}: {
  ariaLabel: string
  format?: (value: number) => string
  max: number
  min: number
  onCommit: (value: number) => void
  step?: number
  value: number
}) {
  const formattedValue = format(value)
  const [draft, setDraft] = useState(formattedValue)
  useEffect(() => setDraft(formattedValue), [formattedValue])
  const commit = () => {
    const parsed = Number(draft)
    if (!Number.isFinite(parsed) || parsed < min || parsed > max) {
      setDraft(formattedValue)
      return
    }
    onCommit(parsed)
  }
  return (
    <input
      aria-label={ariaLabel}
      max={max}
      min={min}
      onBlur={commit}
      onChange={(event) => setDraft(event.target.value)}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit()
        if (event.key === "Escape") setDraft(formattedValue)
      }}
      step={step}
      type="number"
      value={draft}
    />
  )
}

function CliRuntimeControls({
  name,
  onChange,
  value,
  wslDistributions,
}: {
  name: string
  onChange: (value: ProviderCliSettings) => void
  value: ProviderCliSettings
  wslDistributions: string[]
}) {
  return (
    <div className="cli-runtime-controls">
      <select
        aria-label={`${name} runtime`}
        onChange={(event) => {
          const mode = event.target.value as ProviderCliSettings["mode"]
          onChange({ ...value, mode, wslDistribution: mode === "wsl" ? value.wslDistribution : null })
        }}
        value={value.mode}
      >
        <option value="auto">Automatic</option>
        <option value="nativeWindows">Native Windows</option>
        <option value="wsl">WSL</option>
      </select>
      {value.mode === "wsl" ? (
        <select
          aria-label={`${name} WSL distribution`}
          onChange={(event) => onChange({ ...value, wslDistribution: event.target.value || null })}
          value={value.wslDistribution ?? ""}
        >
          <option value="">Choose distribution</option>
          {wslDistributions.map((distribution) => (
            <option key={distribution} value={distribution}>{distribution}</option>
          ))}
        </select>
      ) : (
        <input
          aria-label={`${name} custom CLI path`}
          key={`${name}-${value.customPath ?? "automatic"}`}
          defaultValue={value.customPath ?? ""}
          onBlur={(event) => onChange({ ...value, customPath: event.target.value || null })}
          placeholder="Optional custom CLI path"
          type="text"
        />
      )}
    </div>
  )
}

function updateMessage(state: UpdateState) {
  if (state.phase === "upToDate") return "You’re up to date."
  if (state.phase === "available") return `Version ${state.availableVersion} is available.`
  if (state.phase === "downloading") return state.progressPercent == null
    ? "Downloading signed update…"
    : `Downloading signed update… ${state.progressPercent}%`
  if (state.phase === "installing") return "Installing signed update…"
  if (state.phase === "failed") return state.message ?? "Update check failed."
  return "Updates are checked only when you ask."
}

function SettingRow({ label, hint, children }: { label: string; hint: string; children: ReactNode }) {
  return <section className="setting-row"><div><strong>{label}</strong><small>{hint}</small></div><div>{children}</div></section>
}

function statusFor(statuses: ServiceAccountStatus[], providerId: ProviderId): ServiceAccountStatus {
  return statuses.find((status) => status.providerId === providerId) ?? {
    providerId,
    connectionState: "checking",
  }
}

function Service({ name, status, children }: { name: string; status: ServiceAccountStatus; children: ReactNode }) {
  const source = [status.runtimeSource, status.cliVersion ? `CLI ${status.cliVersion}` : null]
    .filter(Boolean)
    .join(" · ")
  return (
    <article className={`service-card service-card--${status.connectionState}`}>
      <span className="service-identity">
        <strong>{name}</strong>
        <small>{status.accountLabel ?? connectionLabel(status.connectionState, status.providerId)}</small>
        {status.accountDetail ? <small>{status.accountDetail}</small> : null}
        {source ? <small>{source}</small> : null}
      </span>
      <div className="service-actions">{children}</div>
    </article>
  )
}

function connectionLabel(state: ServiceAccountStatus["connectionState"], providerId: ProviderId) {
  if (state === "connected") return "Connected"
  if (state === "signInRequired") return providerId === "deepseek" ? "No API Key stored" : "Sign-in required"
  if (state === "notInstalled") return "CLI not installed"
  if (state === "checking") return "Checking account…"
  return "Account status unavailable"
}

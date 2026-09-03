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
  onReplaceDeepSeekKey?: (candidate: string) => Promise<boolean> | boolean | void
  serviceMessage?: string | null
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
}: SettingsWindowProps) {
  const [activeTab, setActiveTab] = useState<(typeof tabs)[number]>("Appearance")
  const [pendingDeepSeekKey, setPendingDeepSeekKey] = useState("")
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
          <SettingRow label="Detail auto-hide" hint="Interaction pauses the countdown.">
            <input
              aria-label="Detail auto-hide seconds"
              max="300"
              min="1"
              onChange={(event) => onDetailAutoHideSecondsChange(Number(event.target.value))}
              type="number"
              value={detailAutoHideSeconds}
            /> seconds
          </SettingRow>
        ) : null}
        {activeTab === "Services" ? (
          <div className="service-list">
            {(["claude", "codex"] as const).map((providerId) => {
              const name = providerId === "claude" ? "Claude Code" : "OpenAI Codex"
              const status = statusFor(serviceStatuses, providerId)
              return (
                <Service key={providerId} name={name} status={status}>
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
              <input
                aria-label="DeepSeek API Key"
                autoComplete="off"
                onChange={(event) => setPendingDeepSeekKey(event.target.value)}
                placeholder="Enter a replacement API Key"
                type="password"
                value={pendingDeepSeekKey}
              />
              <button
                aria-label="Replace DeepSeek API Key"
                disabled={!pendingDeepSeekKey.trim()}
                onClick={async () => {
                  const result = await onReplaceDeepSeekKey(pendingDeepSeekKey)
                  if (result !== false) setPendingDeepSeekKey("")
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

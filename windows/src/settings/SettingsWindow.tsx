import { useEffect, useState } from "react"
import type { ReactNode } from "react"
import { t, useLocale, setLocale, type Locale } from "../localization"
import { displayFonts, fontAvailable } from "../displayFonts"
import type { ProviderId } from "../state/usage"
import { defaultStripPreferences, type StripPreferences } from "../state/stripPreferences"

const tabs = ["Appearance", "Monitoring", "Services", "About"] as const
const fonts = displayFonts
export type DisplayInfo = { id: string; name: string; isPrimary: boolean }
export type DisplayPreferences = { version: number; mode: "primary" | "selected" | "all"; selectedId: string | null; placements: Record<string, {edge: "left" | "right"; verticalPerMille: number}> }

type SettingsWindowProps = {
  onLocaleChange?: (locale: Locale) => void
  displays?: DisplayPreferences
  availableDisplays?: DisplayInfo[]
  onDisplayModeChange?: (mode: DisplayPreferences["mode"], selectedId: string | null) => void
  stripPreferences?: StripPreferences
  onStripPreferencesChange?: (value: StripPreferences) => void
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
  currentVersion: "0.3.0",
}

export function SettingsWindow({
  onLocaleChange = setLocale,
  displays = {version: 1, mode: "primary", selectedId: null, placements: {}},
  availableDisplays = [],
  onDisplayModeChange = () => {},
  stripPreferences = defaultStripPreferences,
  onStripPreferencesChange = () => {},
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
  const locale = useLocale()
  const primaryDisplay = availableDisplays.find(display => display.isPrimary) ?? availableDisplays[0]
  const selectedDisplay = displays.mode === "selected" ? availableDisplays.find(display => display.id === displays.selectedId) : undefined
  const targetDisplay = selectedDisplay ?? primaryDisplay
  const effectiveEdge = targetDisplay ? displays.placements[targetDisplay.id]?.edge ?? "right" : edge
  const [availableFonts, setAvailableFonts] = useState<Record<string, boolean>>({})
  useEffect(() => { setAvailableFonts(Object.fromEntries(fonts.map(font => [font, fontAvailable(font)]))) }, [])
  const [activeTab, setActiveTab] = useState<(typeof tabs)[number]>("Appearance")
  useEffect(() => {
    if (requestedTab) setActiveTab(requestedTab)
  }, [requestedTab])
  return (
    <section aria-label={t("AI Token Meter Settings")} className="settings-window settings-window--compact-density settings-window--system-font" role="dialog">
      <header><div><strong>{t("AI Token Meter")}</strong><small>{t("Private AI usage, at a glance.")}</small></div></header>
      <nav aria-label={t("Settings categories")} role="tablist">
        {tabs.map((tab) => (
          <button
            aria-selected={activeTab === tab}
            key={tab}
            onClick={() => setActiveTab(tab)}
            role="tab"
            type="button"
          >{t(tab)}</button>
        ))}
      </nav>
      <div className="settings-content">
        {activeTab === "Appearance" ? (
          <>
            <SettingRow label={t("Language")} hint={t("Display language changes immediately in all windows.")}>
              <select aria-label={t("Language")} value={locale} onChange={event => onLocaleChange(event.target.value as Locale)}>
                <option value="en">English</option><option value="zh-CN">简体中文</option>
              </select>
            </SettingRow>
            <SettingRow label={t("Display mode")} hint={t("Choose where the floating strip appears.")}>
              <select aria-label={t("Display mode")} value={displays.mode} onChange={event => onDisplayModeChange(event.target.value as DisplayPreferences["mode"], displays.selectedId)}>
                <option value="primary">{t("Primary display")}</option><option value="selected">{t("Selected display")}</option><option value="all">{t("All displays")}</option>
              </select>
              <button type="button" onClick={() => onDisplayModeChange("primary", null)}>{t("Move to primary display")}</button>
            </SettingRow>
            {displays.mode === "selected" && <SettingRow label={t("Display")} hint={t("An offline selection temporarily uses the primary display.")}>
              <select aria-label={t("Display")} value={displays.selectedId ?? ""} onChange={event => onDisplayModeChange("selected", event.target.value || null)}>
                <option value="">{t("Primary display")}</option>
                {displays.selectedId && !availableDisplays.some(display => display.id === displays.selectedId) && <option value={displays.selectedId}>{t("Offline display")}</option>}
                {availableDisplays.map(display => <option key={display.id} value={display.id}>{display.name}{display.isPrimary ? ` · ${t("Primary")}` : ""}</option>)}
              </select>
            </SettingRow>}
            <SettingRow label={t("Floating strip size")} hint={t("Compact saves space; Comfortable keeps larger rings.")}>
              <select aria-label={t("Floating strip size")} value={stripPreferences.density}
                onChange={e => onStripPreferencesChange({...stripPreferences, density: e.target.value as StripPreferences["density"]})}>
                <option value="compact">{t("Compact")}</option><option value="comfortable">{t("Comfortable")}</option>
              </select>
            </SettingRow>
            <SettingRow label={t("Fold when idle")} hint={t("Interaction, open details and refreshes keep the meter expanded.")}>
              <select aria-label={t("Fold when idle")} value={stripPreferences.foldDelay}
                onChange={e => onStripPreferencesChange({...stripPreferences, foldDelay: Number(e.target.value)})}>
                <option value={0}>{t("Never")}</option><option value={5}>{t("After 5 seconds")}</option><option value={15}>{t("After 15 seconds")}</option>
              </select>
            </SettingRow>
            <SettingRow label={t("Floating strip services")} hint={t("Keep at least one visible. Hidden services continue monitoring.")}>
              <div>
                {stripPreferences.orderedProviders.map((id, index) => {
                  const visible = !stripPreferences.hiddenProviders.includes(id)
                  const label = id === "claude" ? "Claude Code" : id === "codex" ? "OpenAI Codex" : "DeepSeek"
                  const move = (offset: number) => {
                    const order = [...stripPreferences.orderedProviders]
                    ;[order[index], order[index+offset]] = [order[index+offset], order[index]]
                    onStripPreferencesChange({...stripPreferences, orderedProviders: order})
                  }
                  return <div key={id} className="strip-service-row" draggable
                    onDragStart={event => event.dataTransfer.setData("text/plain", id)}
                    onDragOver={event => event.preventDefault()}
                    onDrop={event => {
                      event.preventDefault()
                      const source = event.dataTransfer.getData("text/plain") as typeof id
                      if (source === id || !stripPreferences.orderedProviders.includes(source)) return
                      const order = stripPreferences.orderedProviders.filter(item => item !== source)
                      order.splice(index, 0, source)
                      onStripPreferencesChange({...stripPreferences, orderedProviders: order})
                    }}>
                    <label><input type="checkbox" checked={visible}
                      disabled={visible && stripPreferences.hiddenProviders.length === 2}
                      onChange={e => onStripPreferencesChange({...stripPreferences,
                        hiddenProviders: e.target.checked ? stripPreferences.hiddenProviders.filter(p => p !== id) : [...stripPreferences.hiddenProviders, id]})} />{label}</label>
                    <button type="button" aria-label={t("Move {name} up", {name: label})} disabled={index === 0} onClick={() => move(-1)}>↑</button>
                    <button type="button" aria-label={t("Move {name} down", {name: label})} disabled={index === 2} onClick={() => move(1)}>↓</button>
                  </div>
                })}
                <button type="button" onClick={() => onStripPreferencesChange({...stripPreferences,
                  orderedProviders: defaultStripPreferences.orderedProviders, hiddenProviders: []})}>{t("Restore default order")}</button>
              </div>
            </SettingRow>
            <SettingRow label={t("Display font")} hint={t("Applies to the meter, menu and detail panels. Settings always uses the system font.")}>
              <select aria-label={t("Display font")} onChange={(event) => onDisplayFontChange(event.target.value)} value={displayFont}>
                {fonts.map((font) => <option key={font} value={font}>{t(font)}{availableFonts[font] === false ? ` · ${t("Not installed")}` : ""}</option>)}
              </select>
              <button onClick={() => onDisplayFontChange("Microsoft YaHei")} type="button">{t("Restore default font")}</button>
            </SettingRow>
            <SettingRow label={t("Screen edge")} hint={t("The meter follows the selected display and stays outside the taskbar.")}>
              <select
                aria-label={t("Screen edge")}
                onChange={(event) => onEdgeChange(event.target.value as "left" | "right")}
                value={effectiveEdge}
              ><option value="right">{t("Right")}</option><option value="left">{t("Left")}</option></select>
            </SettingRow>
          </>
        ) : null}
        {activeTab === "Monitoring" ? (
          <>
            <SettingRow label={t("Refresh interval")} hint={t("Scheduled refreshes never overlap; a manual refresh replaces an older task.")}>
              <DraftNumberInput
                ariaLabel={t("Refresh interval seconds")}
                max={86_400}
                min={30}
                onCommit={onRefreshIntervalSecondsChange}
                value={refreshIntervalSeconds}
              /> {t("seconds")}
            </SettingRow>
            <SettingRow label={t("DeepSeek balance baseline")} hint={t("The DeepSeek ring shows the amount consumed from this reference balance.")}>
              ¥ <DraftNumberInput
                ariaLabel={t("DeepSeek balance baseline")}
                format={(value) => String(value / 100)}
                max={1_000_000}
                min={1}
                onCommit={(value) => onDeepSeekBalanceBaselineCentsChange(Math.round(value * 100))}
                step={0.01}
                value={deepseekBalanceBaselineCents}
              />
            </SettingRow>
            <SettingRow label={t("Usage alerts")} hint={t("Notify once at 70% and again at 90%; dropping below 10% re-arms the alerts.")}>
              <input
                aria-label={t("Usage alerts at 70% and 90%")}
                checked={notificationsEnabled}
                onChange={(event) => onNotificationsEnabledChange(event.target.checked)}
                type="checkbox"
              />
            </SettingRow>
            <SettingRow label={t("Launch at login")} hint={t("Start the meter after you sign in to Windows.")}>
              <input
                aria-label={t("Open AI Token Meter at login")}
                checked={launchAtLogin}
                onChange={(event) => onLaunchAtLoginChange(event.target.checked)}
                type="checkbox"
              />
            </SettingRow>
            <SettingRow label={t("Detail auto-hide")} hint={t("Interaction pauses the countdown.")}>
              <DraftNumberInput
                ariaLabel={t("Detail auto-hide seconds")}
                max={300}
                min={1}
                onCommit={onDetailAutoHideSecondsChange}
                value={detailAutoHideSeconds}
              /> {t("seconds")}
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
                    aria-label={`${t(status.connectionState === "connected" ? "Sign in again to" : "Sign in to")} ${name}`}
                    disabled={status.connectionState === "notInstalled" || status.connectionState === "checking"}
                    onClick={() => onBeginServiceSignIn(providerId)}
                    type="button"
                  >{t(status.connectionState === "connected" ? "Sign in again" : "Sign in")}</button>
                  <button
                    aria-label={t("Check {name} status", {name})}
                    disabled={status.connectionState === "checking"}
                    onClick={() => onCheckServiceStatus(providerId)}
                    type="button"
                  >{t("Check Status")}</button>
                </Service>
              )
            })}
            <Service name="DeepSeek" status={statusFor(serviceStatuses, "deepseek")}>
              <small>{t("Windows opens a protected credential prompt; the Key never enters this WebView.")}</small>
              <button
                aria-label={t("Replace DeepSeek API Key")}
                onClick={async () => {
                  await onReplaceDeepSeekKey()
                }}
                type="button"
              >{t("Replace API Key")}</button>
              <button
                aria-label={t("Check DeepSeek status")}
                onClick={() => onCheckServiceStatus("deepseek")}
                type="button"
              >{t("Check Status")}</button>
            </Service>
            {serviceMessage ? <p aria-live="polite" className="service-message">{t(serviceMessage)}</p> : null}
          </div>
        ) : null}
        {activeTab === "About" ? (
          <div className="about-card">
            <strong>{t("AI Token Meter")}</strong>
            <p>{t("Version")} {updateState.currentVersion}</p>
            <p>{t("Author · Miller")}</p>
            <p aria-live="polite" className="update-status">{t(updateMessage(updateState))}</p>
            <div className="update-actions">
              <button
                disabled={["checking", "downloading", "installing"].includes(updateState.phase)}
                onClick={onCheckForUpdates}
                type="button"
              >{t(updateState.phase === "checking" ? "Checking…" : "Check for Updates")}</button>
              <button
                disabled={updateState.phase !== "available"}
                onClick={onInstallUpdate}
                type="button"
              >{t(updateState.phase === "downloading" || updateState.phase === "installing" ? "Installing…" : "Update Now")}</button>
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
    const stepOffset = (parsed - min) / step
    const stepTolerance = Number.EPSILON * Math.max(1, Math.abs(stepOffset)) * 16
    const followsStep = Math.abs(stepOffset - Math.round(stepOffset)) <= stepTolerance
    if (!Number.isFinite(parsed) || parsed < min || parsed > max || !followsStep) {
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
        aria-label={t("{name} runtime", {name})}
        onChange={(event) => {
          const mode = event.target.value as ProviderCliSettings["mode"]
          onChange({ ...value, mode, wslDistribution: mode === "wsl" ? value.wslDistribution : null })
        }}
        value={value.mode}
      >
        <option value="auto">{t("Automatic")}</option>
        <option value="nativeWindows">{t("Native Windows")}</option>
        <option value="wsl">{t("WSL")}</option>
      </select>
      {value.mode === "wsl" ? (
        <select
          aria-label={t("{name} WSL distribution", {name})}
          onChange={(event) => onChange({ ...value, wslDistribution: event.target.value || null })}
          value={value.wslDistribution ?? ""}
        >
          <option value="">{t("Choose distribution")}</option>
          {wslDistributions.map((distribution) => (
            <option key={distribution} value={distribution}>{distribution}</option>
          ))}
        </select>
      ) : (
        <input
          aria-label={t("{name} custom CLI path", {name})}
          key={`${name}-${value.customPath ?? "automatic"}`}
          defaultValue={value.customPath ?? ""}
          onBlur={(event) => onChange({ ...value, customPath: event.target.value || null })}
          placeholder={t("Optional custom CLI path")}
          type="text"
        />
      )}
    </div>
  )
}

function updateMessage(state: UpdateState) {
  if (state.phase === "upToDate") return "You’re up to date."
  if (state.phase === "available") return t("Version {version} is available.", {version: state.availableVersion ?? ""})
  if (state.phase === "downloading") return state.progressPercent == null
    ? "Downloading signed update…"
    : `${t("Downloading signed update…")} ${state.progressPercent}%`
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
  const source = [status.runtimeSource ? t(status.runtimeSource) : null, status.cliVersion ? `CLI ${status.cliVersion}` : null]
    .filter(Boolean)
    .join(" · ")
  return (
    <article className={`service-card service-card--${status.connectionState}`}>
      <span className="service-identity">
        <strong>{name}</strong>
        <small>{t(status.accountLabel ?? connectionLabel(status.connectionState, status.providerId))}</small>
        {status.accountDetail ? <small>{t(status.accountDetail)}</small> : null}
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

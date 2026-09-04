import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useEffect, useMemo, useState } from "react"
import type { CSSProperties } from "react"

import { App } from "./App"
import { FloatingStrip } from "./components/FloatingStrip"
import { type DeepSeekHistoryStatus } from "./details/DeepSeekDetail"
import { ProviderDetail } from "./details/ProviderDetail"
import {
  SettingsWindow,
  type ProviderCliSettings,
  type ServiceAccountStatus,
  type UpdateState,
} from "./settings/SettingsWindow"
import type { ProviderId, UsageSnapshot } from "./state/usage"
import { TauriUsageBridge } from "./state/usageBridge"
import { useUsageSnapshots } from "./state/useUsageSnapshots"

type RuntimeSettings = {
  displayFont: string
  edge: "left" | "right"
  detailAutoHideSeconds: number
  refreshIntervalSeconds: number
  deepseekBalanceBaselineCents: number
  notificationsEnabled: boolean
  launchAtLogin: boolean
  claudeCli: ProviderCliSettings
  codexCli: ProviderCliSettings
}

const defaultSettings: RuntimeSettings = {
  displayFont: "Antonio",
  edge: "right",
  detailAutoHideSeconds: 8,
  refreshIntervalSeconds: 300,
  deepseekBalanceBaselineCents: 10_000,
  notificationsEnabled: false,
  launchAtLogin: false,
  claudeCli: { mode: "auto", customPath: null, wslDistribution: null },
  codexCli: { mode: "auto", customPath: null, wslDistribution: null },
}

const deepseekHistoryStatuses = new Set<DeepSeekHistoryStatus>([
  "idle", "opening", "active", "completed", "cancelled", "failed",
])

export function Shell() {
  const label = currentWindowLabel()
  if (label === "meter") return <MeterSurface />
  if (label === "detail") return <DetailSurface />
  if (label === "settings") return <SettingsSurface />
  return <App />
}

function MeterSurface() {
  const bridge = useMemo(() => new TauriUsageBridge(), [])
  const snapshots = useUsageSnapshots(bridge)
  const [activeProvider, setActiveProvider] = useState<ProviderId | null>(null)
  const settings = useRuntimeSettings()
  useEffect(() => {
    const startDrag = () => {
      void invoke("begin_meter_drag")
        .then(() => getCurrentWindow().startDragging())
        .catch(() => {})
    }
    window.addEventListener("meter-drag-requested", startDrag)
    return () => window.removeEventListener("meter-drag-requested", startDrag)
  }, [])
  useEffect(() => {
    let disposed = false
    let stop: (() => void) | undefined
    listen("detail-closed", () => {
      if (!disposed) setActiveProvider(null)
    }).then((unlisten) => {
      if (disposed) unlisten()
      else stop = unlisten
    }).catch(() => {})
    return () => {
      disposed = true
      stop?.()
    }
  }, [])
  return (
    <main
      className={`meter-stage meter-stage--strip-only meter-edge--${settings.edge}`}
      style={displayStyle(settings.displayFont)}
    >
      <FloatingStrip
        activeProvider={activeProvider}
        onProviderActivate={(providerId) => {
          if (providerId === activeProvider) {
            setActiveProvider(null)
            void invoke("close_provider_detail")
            return
          }
          setActiveProvider(providerId)
          void invoke("show_provider_detail", { providerId }).catch(() => setActiveProvider(null))
        }}
        snapshots={snapshots}
      />
    </main>
  )
}

export function DetailSurface() {
  const [snapshot, setSnapshot] = useState<UsageSnapshot | null>(null)
  const [paused, setPaused] = useState(false)
  const [deepseekHistoryStatus, setDeepseekHistoryStatus] = useState<DeepSeekHistoryStatus>("idle")
  const settings = useRuntimeSettings()

  useEffect(() => {
    let disposed = false
    const stops: Array<() => void> = []
    const active = listen<UsageSnapshot>("active-detail-changed", (event) => {
      if (!disposed) setSnapshot(event.payload)
    })
    const refreshed = listen<UsageSnapshot>("snapshot-updated", (event) => {
      if (!disposed) {
        setSnapshot((current) => current?.providerId === event.payload.providerId ? event.payload : current)
      }
    })
    const historyStatus = listen<unknown>("deepseek-history-status", (event) => {
      if (!disposed && isDeepSeekHistoryStatus(event.payload)) setDeepseekHistoryStatus(event.payload)
    })
    void Promise.all([active, refreshed, historyStatus]).then((unlisten) => {
      if (disposed) unlisten.forEach((stop) => stop())
      else stops.push(...unlisten)
    }).catch(() => {})
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [])

  useEffect(() => {
    const syncingHistory = snapshot?.providerId === "deepseek"
      && (deepseekHistoryStatus === "opening" || deepseekHistoryStatus === "active")
    if (!snapshot || paused || syncingHistory) return
    const timeout = window.setTimeout(() => {
      setSnapshot(null)
      void invoke("close_provider_detail")
    }, settings.detailAutoHideSeconds * 1_000)
    return () => window.clearTimeout(timeout)
  }, [deepseekHistoryStatus, paused, settings.detailAutoHideSeconds, snapshot])

  if (!snapshot) return null
  return (
    <main className="detail-surface" style={displayStyle(settings.displayFont)}>
      <ProviderDetail
        onInteractionEnd={() => setPaused(false)}
        onInteractionStart={() => setPaused(true)}
        onDeepSeekHistorySync={() => {
          setDeepseekHistoryStatus("opening")
          void invoke("open_deepseek_history").catch(() => setDeepseekHistoryStatus("failed"))
        }}
        onPointerEnter={() => setPaused(true)}
        onPointerLeave={() => setPaused(false)}
        snapshot={snapshot}
        deepseekHistoryStatus={deepseekHistoryStatus}
      />
    </main>
  )
}

function SettingsSurface() {
  const settings = useRuntimeSettings()
  const [updateState, setUpdateState] = useState<UpdateState>({ phase: "idle", currentVersion: "0.3.0-preview.2" })
  const [serviceStatuses, setServiceStatuses] = useState<ServiceAccountStatus[]>([])
  const [serviceMessage, setServiceMessage] = useState<string | null>(null)
  const [wslDistributions, setWslDistributions] = useState<string[]>([])
  const [requestedTab, setRequestedTab] = useState<"Appearance" | "Monitoring" | "Services" | "About">()
  useEffect(() => {
    let disposed = false
    let stop: (() => void) | undefined
    listen<"Appearance" | "Monitoring" | "Services" | "About">("settings-tab-requested", (event) => {
      if (!disposed) setRequestedTab(event.payload)
    }).then((unlisten) => {
      if (disposed) unlisten()
      else stop = unlisten
    }).catch(() => {})
    return () => {
      disposed = true
      stop?.()
    }
  }, [])
  useEffect(() => {
    let disposed = false
    invoke<string[]>("available_wsl_distributions").then((distributions) => {
      if (!disposed) setWslDistributions(distributions)
    }).catch(() => {})
    return () => { disposed = true }
  }, [])
  useEffect(() => {
    let disposed = false
    invoke<ServiceAccountStatus[]>("service_account_statuses").then((statuses) => {
      if (!disposed) setServiceStatuses(statuses)
    }).catch(() => {
      if (!disposed) setServiceMessage("Account status is temporarily unavailable.")
    })
    return () => { disposed = true }
  }, [])
  const applyServiceStatus = (status: ServiceAccountStatus) => {
    setServiceStatuses((current) => [
      ...current.filter((item) => item.providerId !== status.providerId),
      status,
    ])
  }
  const checkServiceStatus = (providerId: ProviderId) => {
    applyServiceStatus({ providerId, connectionState: "checking" })
    setServiceMessage(null)
    void invoke<ServiceAccountStatus>("service_account_status", { providerId })
      .then(applyServiceStatus)
      .catch(() => {
        applyServiceStatus({ providerId, connectionState: "unavailable" })
        setServiceMessage("The account status check did not complete.")
      })
  }
  useEffect(() => {
    let disposed = false
    let stop: (() => void) | undefined
    invoke<UpdateState>("update_state").then((value) => {
      if (!disposed) setUpdateState(value)
    }).catch(() => {})
    listen<UpdateState>("update-state-changed", (event) => {
      if (!disposed) setUpdateState(event.payload)
    }).then((unlisten) => {
      if (disposed) unlisten()
      else stop = unlisten
    }).catch(() => {})
    return () => {
      disposed = true
      stop?.()
    }
  }, [])
  return (
    <SettingsWindow
      detailAutoHideSeconds={settings.detailAutoHideSeconds}
      refreshIntervalSeconds={settings.refreshIntervalSeconds}
      deepseekBalanceBaselineCents={settings.deepseekBalanceBaselineCents}
      notificationsEnabled={settings.notificationsEnabled}
      launchAtLogin={settings.launchAtLogin}
      displayFont={settings.displayFont}
      edge={settings.edge}
      onDetailAutoHideSecondsChange={(seconds) => {
        if (Number.isInteger(seconds) && seconds >= 1 && seconds <= 300) {
          void invoke("set_detail_auto_hide_seconds", { seconds })
        }
      }}
      onRefreshIntervalSecondsChange={(seconds) => {
        if (Number.isInteger(seconds) && seconds >= 30 && seconds <= 86_400) {
          void invoke("set_refresh_interval_seconds", { seconds })
        }
      }}
      onDeepSeekBalanceBaselineCentsChange={(cents) => {
        if (Number.isInteger(cents) && cents >= 100 && cents <= 100_000_000) {
          void invoke("set_deepseek_balance_baseline_cents", { cents })
        }
      }}
      onNotificationsEnabledChange={(enabled) => {
        void invoke("set_notifications_enabled", { enabled })
      }}
      onLaunchAtLoginChange={(enabled) => {
        setServiceMessage(null)
        void invoke("set_launch_at_login", { enabled })
          .catch(() => setServiceMessage("Launch at login could not be changed."))
      }}
      onDisplayFontChange={(font) => void invoke("set_display_font", { font })}
      onEdgeChange={(nextEdge) => {
        void invoke("set_meter_edge", { edge: nextEdge })
      }}
      requestedTab={requestedTab}
      updateState={updateState}
      onCheckForUpdates={() => void invoke<UpdateState>("check_for_updates").then(setUpdateState).catch(() => {})}
      onInstallUpdate={() => void invoke("install_update").catch(() => {})}
      serviceStatuses={serviceStatuses}
      serviceMessage={serviceMessage}
      cliSettings={{ claude: settings.claudeCli, codex: settings.codexCli }}
      wslDistributions={wslDistributions}
      onCliSettingsChange={(providerId, value) => {
        setServiceMessage("Saving CLI runtime and refreshing this service…")
        void invoke<RuntimeSettings>("set_provider_cli_settings", { providerId, value })
          .then(() => {
            setServiceMessage("CLI runtime saved.")
            checkServiceStatus(providerId)
          })
          .catch(() => setServiceMessage("The CLI runtime setting could not be saved."))
      }}
      onCheckServiceStatus={checkServiceStatus}
      onBeginServiceSignIn={(providerId) => {
        setServiceMessage(null)
        void invoke("begin_service_sign_in", { providerId }).then(() => {
          setServiceMessage("Complete sign-in in the new terminal window, then choose Check Status.")
        }).catch(() => {
          setServiceMessage("The sign-in window could not be opened.")
        })
      }}
      onReplaceDeepSeekKey={async () => {
        setServiceMessage("Open the protected Windows prompt to replace the API Key.")
        try {
          const status = await invoke<ServiceAccountStatus>("replace_deepseek_api_key")
          applyServiceStatus(status)
          setServiceMessage("DeepSeek accepted the replacement API Key.")
          return true
        } catch {
          setServiceMessage("The replacement was not saved. The existing API Key remains active.")
          return false
        }
      }}
    />
  )
}

function useRuntimeSettings() {
  const [settings, setSettings] = useState<RuntimeSettings>(defaultSettings)
  useEffect(() => {
    let disposed = false
    const stops: Array<() => void> = []
    invoke<RuntimeSettings>("app_settings").then((value) => {
      if (!disposed) setSettings(value)
    }).catch(() => {})
    const subscriptions = [
      listen<"left" | "right">("meter-edge-changed", (event) => {
        if (!disposed) setSettings((current) => ({ ...current, edge: event.payload }))
      }),
      listen<string>("display-font-changed", (event) => {
        if (!disposed) setSettings((current) => ({ ...current, displayFont: event.payload }))
      }),
      listen<number>("detail-auto-hide-changed", (event) => {
        if (!disposed) setSettings((current) => ({ ...current, detailAutoHideSeconds: event.payload }))
      }),
      listen<RuntimeSettings>("app-settings-changed", (event) => {
        if (!disposed) setSettings(event.payload)
      }),
    ]
    void Promise.all(subscriptions).then((unlisten) => {
      if (disposed) unlisten.forEach((stop) => stop())
      else stops.push(...unlisten)
    }).catch(() => {})
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [])
  return settings
}

function displayStyle(font: string) {
  const stack = font === "System Default"
    ? "'Segoe UI Variable', 'Segoe UI', sans-serif"
    : `'${font.replaceAll("'", "")}', 'Segoe UI Variable', sans-serif`
  return { "--display-font": stack } as CSSProperties
}

function currentWindowLabel() {
  return "__TAURI_INTERNALS__" in window ? getCurrentWindow().label : "preview"
}

function isDeepSeekHistoryStatus(value: unknown): value is DeepSeekHistoryStatus {
  return typeof value === "string" && deepseekHistoryStatuses.has(value as DeepSeekHistoryStatus)
}

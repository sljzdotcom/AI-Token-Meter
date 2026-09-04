import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
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

type DeepSeekHistoryStatusSnapshot = {
  generation: number | null
  status: DeepSeekHistoryStatus
}

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
  const [deepseekHistoryState, setDeepseekHistoryState] = useState<DeepSeekHistoryStatusSnapshot>({
    generation: null,
    status: "idle",
  })
  const [deepseekHistoryStatusPathAvailable, setDeepseekHistoryStatusPathAvailable] = useState(false)
  const deepseekHistoryGeneration = useRef<number | null>(null)
  const deepseekHistoryGenerationFloor = useRef<number | null>(null)
  const deepseekHistoryCurrent = useRef<DeepSeekHistoryStatusSnapshot>({
    generation: null,
    status: "idle",
  })
  const deepseekHistoryAttempt = useRef(0)
  const settings = useRuntimeSettings()

  const acceptDeepSeekHistoryStatus = useCallback((value: unknown, authoritative = false) => {
    if (!isDeepSeekHistoryStatusSnapshot(value)) return false
    const current = deepseekHistoryGeneration.current
    const floor = deepseekHistoryGenerationFloor.current
    if (authoritative) {
      if (current != null && value.generation == null) return false
      if (current != null && value.generation != null && value.generation < current) return false
      if (current === value.generation
        && historyStatusRank(value.status) < historyStatusRank(deepseekHistoryCurrent.current.status)) {
        return true
      }
      deepseekHistoryGeneration.current = value.generation
      deepseekHistoryCurrent.current = value
      setDeepseekHistoryState(value)
      return true
    }
    if (value.generation == null) return false
    if (current == null && floor != null && value.generation <= floor) return false
    if (current == null) {
      if (value.status !== "opening" && value.status !== "active") return false
      deepseekHistoryGeneration.current = value.generation
    } else if (value.generation !== current) {
      return false
    }
    if (current === value.generation
      && historyStatusRank(value.status) < historyStatusRank(deepseekHistoryCurrent.current.status)) {
      return true
    }
    deepseekHistoryCurrent.current = value
    setDeepseekHistoryState(value)
    return true
  }, [])

  useEffect(() => {
    let disposed = false
    const stops: Array<() => void> = []
    const subscribe = <T,>(event: string, handler: (event: { payload: T }) => void) => {
      void listen<T>(event, handler).then((stop) => {
        if (disposed) stop()
        else stops.push(stop)
      }).catch(() => {})
    }
    subscribe<UsageSnapshot>("active-detail-changed", (event) => {
      if (!disposed) setSnapshot(event.payload)
    })
    subscribe<UsageSnapshot>("snapshot-updated", (event) => {
      if (!disposed) {
        setSnapshot((current) => current?.providerId === event.payload.providerId ? event.payload : current)
      }
    })
    void listen<unknown>("deepseek-history-status", (event) => {
      if (!disposed) acceptDeepSeekHistoryStatus(event.payload)
    }).then((stop) => {
      if (disposed) {
        stop()
        return
      }
      stops.push(stop)
      setDeepseekHistoryStatusPathAvailable(true)
      const initialAttempt = deepseekHistoryAttempt.current
      void invoke<unknown>("deepseek_history_status").then((value) => {
        if (!disposed
          && deepseekHistoryAttempt.current === initialAttempt
          && deepseekHistoryGeneration.current == null) {
          acceptDeepSeekHistoryStatus(value, true)
        }
      }).catch(() => {})
    }).catch(() => {
      const initialAttempt = deepseekHistoryAttempt.current
      void invoke<unknown>("deepseek_history_status").then((value) => {
        if (disposed
          || deepseekHistoryAttempt.current !== initialAttempt
          || !isDeepSeekHistoryStatusSnapshot(value)) return
        setDeepseekHistoryStatusPathAvailable(true)
        acceptDeepSeekHistoryStatus(value, true)
      }).catch(() => {
        if (!disposed && deepseekHistoryAttempt.current === initialAttempt) {
          setDeepseekHistoryStatusPathAvailable(false)
        }
      })
    })
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [acceptDeepSeekHistoryStatus])

  useEffect(() => {
    if (deepseekHistoryState.status !== "opening" && deepseekHistoryState.status !== "active") {
      return
    }
    let disposed = false
    const poll = () => {
      void invoke<unknown>("deepseek_history_status").then((value) => {
        if (!disposed) acceptDeepSeekHistoryStatus(value)
      }).catch(() => {})
    }
    const interval = window.setInterval(poll, 500)
    return () => {
      disposed = true
      window.clearInterval(interval)
    }
  }, [acceptDeepSeekHistoryStatus, deepseekHistoryState.status])

  useEffect(() => {
    const syncingHistory = snapshot?.providerId === "deepseek"
      && (deepseekHistoryState.status === "opening" || deepseekHistoryState.status === "active")
    if (!snapshot || paused || syncingHistory) return
    const timeout = window.setTimeout(() => {
      setSnapshot(null)
      void invoke("close_provider_detail")
    }, settings.detailAutoHideSeconds * 1_000)
    return () => window.clearTimeout(timeout)
  }, [deepseekHistoryState.status, paused, settings.detailAutoHideSeconds, snapshot])

  if (!snapshot) return null
  return (
    <main className="detail-surface" style={displayStyle(settings.displayFont)}>
      <ProviderDetail
        onInteractionEnd={() => setPaused(false)}
        onInteractionStart={() => setPaused(true)}
        onDeepSeekHistorySync={() => {
          if (!deepseekHistoryStatusPathAvailable) return
          const attempt = ++deepseekHistoryAttempt.current
          const previousGeneration = deepseekHistoryGeneration.current
          if (previousGeneration != null) {
            deepseekHistoryGenerationFloor.current = Math.max(
              deepseekHistoryGenerationFloor.current ?? 0,
              previousGeneration,
            )
          }
          deepseekHistoryGeneration.current = null
          const opening: DeepSeekHistoryStatusSnapshot = { generation: null, status: "opening" }
          deepseekHistoryCurrent.current = opening
          setDeepseekHistoryState(opening)
          const hasConfirmedLiveSession = () => deepseekHistoryGeneration.current != null
            && (deepseekHistoryCurrent.current.status === "opening"
              || deepseekHistoryCurrent.current.status === "active")
          const failUnlessLive = () => {
            if (deepseekHistoryAttempt.current !== attempt || hasConfirmedLiveSession()) return
            const failed: DeepSeekHistoryStatusSnapshot = { generation: null, status: "failed" }
            deepseekHistoryCurrent.current = failed
            setDeepseekHistoryState(failed)
          }
          void invoke<unknown>("open_deepseek_history").then((value) => {
            if (deepseekHistoryAttempt.current !== attempt) return
            const accepted = acceptDeepSeekHistoryStatus(value, true)
            if (!accepted
              || !isDeepSeekHistoryStatusSnapshot(value)
              || value.generation == null) {
              failUnlessLive()
            }
          }).catch(() => {
            if (deepseekHistoryAttempt.current !== attempt) return
            void invoke<unknown>("deepseek_history_status").then((value) => {
              if (deepseekHistoryAttempt.current !== attempt) return
              if (isDeepSeekHistoryStatusSnapshot(value)) {
                const accepted = acceptDeepSeekHistoryStatus(value, true)
                if (accepted && value.generation != null) return
              }
              failUnlessLive()
            }).catch(() => {
              failUnlessLive()
            })
          })
        }}
        onPointerEnter={() => setPaused(true)}
        onPointerLeave={() => setPaused(false)}
        snapshot={snapshot}
        deepseekHistoryStatus={deepseekHistoryState.status}
        deepseekHistoryStatusPathAvailable={deepseekHistoryStatusPathAvailable}
      />
    </main>
  )
}

function SettingsSurface() {
  const settings = useRuntimeSettings()
  const [updateState, setUpdateState] = useState<UpdateState>({ phase: "idle", currentVersion: "0.3.0-preview.3" })
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

function isDeepSeekHistoryStatusSnapshot(value: unknown): value is DeepSeekHistoryStatusSnapshot {
  if (!value || typeof value !== "object") return false
  const candidate = value as Partial<DeepSeekHistoryStatusSnapshot>
  return (candidate.generation === null
      || (typeof candidate.generation === "number"
        && Number.isSafeInteger(candidate.generation)
        && candidate.generation > 0))
    && typeof candidate.status === "string"
    && deepseekHistoryStatuses.has(candidate.status as DeepSeekHistoryStatus)
}

function historyStatusRank(status: DeepSeekHistoryStatus) {
  if (status === "idle") return 0
  if (status === "opening") return 1
  if (status === "active") return 2
  return 3
}

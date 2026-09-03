import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useEffect, useMemo, useState } from "react"
import type { CSSProperties } from "react"

import { App } from "./App"
import { FloatingStrip } from "./components/FloatingStrip"
import { ProviderDetail } from "./details/ProviderDetail"
import { SettingsWindow, type ServiceAccountStatus, type UpdateState } from "./settings/SettingsWindow"
import type { ProviderId, UsageSnapshot } from "./state/usage"
import { TauriUsageBridge } from "./state/usageBridge"
import { useUsageSnapshots } from "./state/useUsageSnapshots"

type RuntimeSettings = {
  displayFont: string
  edge: "left" | "right"
  detailAutoHideSeconds: number
}

const defaultSettings: RuntimeSettings = {
  displayFont: "Antonio",
  edge: "right",
  detailAutoHideSeconds: 8,
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
      void getCurrentWindow().startDragging().then(() => invoke("meter_drag_ended"))
    }
    window.addEventListener("meter-drag-requested", startDrag)
    return () => window.removeEventListener("meter-drag-requested", startDrag)
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

function DetailSurface() {
  const [snapshot, setSnapshot] = useState<UsageSnapshot | null>(null)
  const [paused, setPaused] = useState(false)
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
    void Promise.all([active, refreshed]).then((unlisten) => {
      if (disposed) unlisten.forEach((stop) => stop())
      else stops.push(...unlisten)
    }).catch(() => {})
    return () => {
      disposed = true
      stops.forEach((stop) => stop())
    }
  }, [])

  useEffect(() => {
    if (!snapshot || paused) return
    const timeout = window.setTimeout(() => {
      setSnapshot(null)
      void invoke("close_provider_detail")
    }, settings.detailAutoHideSeconds * 1_000)
    return () => window.clearTimeout(timeout)
  }, [paused, settings.detailAutoHideSeconds, snapshot])

  if (!snapshot) return null
  return (
    <main className="detail-surface" style={displayStyle(settings.displayFont)}>
      <ProviderDetail
        onInteractionEnd={() => setPaused(false)}
        onInteractionStart={() => setPaused(true)}
        onDeepSeekHistorySync={() => void invoke("open_deepseek_history")}
        onPointerEnter={() => setPaused(true)}
        onPointerLeave={() => setPaused(false)}
        snapshot={snapshot}
      />
    </main>
  )
}

function SettingsSurface() {
  const settings = useRuntimeSettings()
  const [updateState, setUpdateState] = useState<UpdateState>({ phase: "idle", currentVersion: "0.2.2" })
  const [serviceStatuses, setServiceStatuses] = useState<ServiceAccountStatus[]>([])
  const [serviceMessage, setServiceMessage] = useState<string | null>(null)
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
      displayFont={settings.displayFont}
      edge={settings.edge}
      onDetailAutoHideSecondsChange={(seconds) => {
        if (Number.isInteger(seconds) && seconds >= 1 && seconds <= 300) {
          void invoke("set_detail_auto_hide_seconds", { seconds })
        }
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

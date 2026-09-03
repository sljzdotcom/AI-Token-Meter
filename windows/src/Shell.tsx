import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useEffect, useMemo, useState } from "react"
import type { CSSProperties } from "react"

import { App } from "./App"
import { FloatingStrip } from "./components/FloatingStrip"
import { ProviderDetail } from "./details/ProviderDetail"
import { SettingsWindow } from "./settings/SettingsWindow"
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

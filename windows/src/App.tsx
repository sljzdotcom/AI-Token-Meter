import { useEffect, useMemo, useRef, useState } from "react"
import type { CSSProperties } from "react"

import { FloatingStrip } from "./components/FloatingStrip"
import { ProviderDetail } from "./details/ProviderDetail"
import { Code0Detail } from "./details/Code0Detail"
import type { ProviderId, UsageSnapshot } from "./state/usage"
import type { UsageBridge } from "./state/usageBridge"
import { TauriUsageBridge } from "./state/usageBridge"
import { useUsageSnapshots } from "./state/useUsageSnapshots"
import { disabledCode0SessionBridge, type Code0SessionBridge } from "./state/code0Session"
import { TauriCode0SessionBridge } from "./state/tauriCode0SessionBridge"
import { useCode0Session } from "./state/useCode0Session"
import type { StripPreferences } from "./state/stripPreferences"

type AppProps = {
  initialSnapshots?: UsageSnapshot[]
  detailAutoHideSeconds?: number
  usageBridge?: UsageBridge
  code0Bridge?: Code0SessionBridge
  stripPreferences?: StripPreferences
  onDeepSeekHistorySync?: () => void
}

export function App({ initialSnapshots, detailAutoHideSeconds = 8, usageBridge, code0Bridge, stripPreferences, onDeepSeekHistorySync }: AppProps) {
  const [activeProvider, setActiveProvider] = useState<ProviderId | null>(null)
  const [interactionPaused, setInteractionPaused] = useState(false)
  const detailRef = useRef<HTMLDivElement>(null)
  const bridge = useMemo(() => usageBridge ?? new TauriUsageBridge(), [usageBridge])
  const sessionBridge = useMemo(() => code0Bridge ?? ("__TAURI_INTERNALS__" in window ? new TauriCode0SessionBridge() : disabledCode0SessionBridge), [code0Bridge])
  const session = useCode0Session(sessionBridge)
  const snapshots = useUsageSnapshots(bridge, initialSnapshots, session)
  const activeSnapshot = snapshots.find((snapshot) => snapshot.providerId === activeProvider)

  useEffect(() => {
    if (!activeProvider || interactionPaused) return
    const timer = window.setTimeout(() => setActiveProvider(null), detailAutoHideSeconds * 1_000)
    return () => window.clearTimeout(timer)
  }, [activeProvider, detailAutoHideSeconds, interactionPaused])

  useEffect(() => {
    if (!activeProvider) return
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (!detailRef.current?.contains(event.target as Node)) setActiveProvider(null)
    }
    document.addEventListener("pointerdown", closeOnOutsidePointer)
    return () => document.removeEventListener("pointerdown", closeOnOutsidePointer)
  }, [activeProvider])

  return (
    <main className="meter-stage" style={{ "--display-font": "Antonio, 'Segoe UI Variable', sans-serif" } as CSSProperties}>
      <FloatingStrip
        preferences={stripPreferences}
        activeProvider={activeProvider}
        onProviderActivate={(provider) => {
          setInteractionPaused(false)
          setActiveProvider((current) => current === provider ? null : provider)
        }}
        snapshots={snapshots}
      />
      {activeProvider === "code0" || activeSnapshot ? (
        <div className="detail-anchor" ref={detailRef}
          onPointerEnter={() => setInteractionPaused(true)} onPointerLeave={() => setInteractionPaused(false)}>
          {activeProvider === "code0" ? <Code0Detail session={session} /> : activeSnapshot ? <ProviderDetail
            onInteractionEnd={() => setInteractionPaused(false)}
            onInteractionStart={() => setInteractionPaused(true)}
            onDeepSeekHistorySync={onDeepSeekHistorySync}
            onPointerEnter={() => setInteractionPaused(true)}
            onPointerLeave={() => setInteractionPaused(false)}
            snapshot={activeSnapshot}
          /> : null}
        </div>
      ) : null}
    </main>
  )
}

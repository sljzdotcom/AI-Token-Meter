import { useEffect, useMemo, useRef, useState } from "react"
import type { CSSProperties } from "react"

import { FloatingStrip } from "./components/FloatingStrip"
import { ProviderDetail } from "./details/ProviderDetail"
import type { ProviderId, UsageSnapshot } from "./state/usage"
import type { UsageBridge } from "./state/usageBridge"
import { TauriUsageBridge } from "./state/usageBridge"
import { useUsageSnapshots } from "./state/useUsageSnapshots"

type AppProps = {
  initialSnapshots?: UsageSnapshot[]
  detailAutoHideSeconds?: number
  usageBridge?: UsageBridge
}

export function App({ initialSnapshots, detailAutoHideSeconds = 8, usageBridge }: AppProps) {
  const [activeProvider, setActiveProvider] = useState<ProviderId | null>(null)
  const [interactionPaused, setInteractionPaused] = useState(false)
  const detailRef = useRef<HTMLDivElement>(null)
  const bridge = useMemo(() => usageBridge ?? new TauriUsageBridge(), [usageBridge])
  const snapshots = useUsageSnapshots(bridge, initialSnapshots)
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
        activeProvider={activeProvider}
        onProviderActivate={(provider) => {
          setInteractionPaused(false)
          setActiveProvider((current) => current === provider ? null : provider)
        }}
        snapshots={snapshots}
      />
      {activeSnapshot ? (
        <div className="detail-anchor" ref={detailRef}>
          <ProviderDetail
            onInteractionEnd={() => setInteractionPaused(false)}
            onInteractionStart={() => setInteractionPaused(true)}
            onPointerEnter={() => setInteractionPaused(true)}
            onPointerLeave={() => setInteractionPaused(false)}
            snapshot={activeSnapshot}
          />
        </div>
      ) : null}
    </main>
  )
}

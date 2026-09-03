import type { ProviderId, UsageSnapshot } from "../state/usage"
import { UsageRing } from "./UsageRing"

type FloatingStripProps = {
  snapshots: UsageSnapshot[]
  activeProvider: ProviderId | null
  onProviderActivate: (provider: ProviderId) => void
}

export function FloatingStrip({ snapshots, activeProvider, onProviderActivate }: FloatingStripProps) {
  return (
    <nav
      aria-label="AI usage providers"
      className="floating-strip"
      onPointerDown={(event) => {
        if (!(event.target as HTMLElement).closest("button")) {
          window.dispatchEvent(new CustomEvent("meter-drag-requested"))
        }
      }}
    >
      <div aria-hidden="true" className="floating-strip__drag-handle" />
      {snapshots.map((snapshot) => (
        <UsageRing
          key={snapshot.providerId}
          onActivate={() => onProviderActivate(snapshot.providerId)}
          selected={activeProvider === snapshot.providerId}
          snapshot={snapshot}
        />
      ))}
    </nav>
  )
}

import type { ProviderId, UsageSnapshot } from "../state/usage"
import { UsageRing } from "./UsageRing"

type FloatingStripProps = {
  snapshots: UsageSnapshot[]
  activeProvider: ProviderId | null
  onProviderActivate: (provider: ProviderId) => void
}

export function FloatingStrip({ snapshots, activeProvider, onProviderActivate }: FloatingStripProps) {
  return (
    <>
      <MeterClipPaths />
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
    </>
  )
}

export function MeterClipPaths() {
  return (
    <svg aria-hidden="true" className="meter-clip-paths" focusable="false">
      <defs>
        <clipPath id="meter-clip-right" clipPathUnits="objectBoundingBox">
          <path d="M 1 0.0449438 C 0.9074074 0.0646067 0.8148148 0.0758427 0.6111111 0.0786517 C 0.2685185 0.0814607 0 0.1516854 0 0.247191 L 0 0.752809 C 0 0.8483146 0.2685185 0.9185393 0.6111111 0.9213483 C 0.8148148 0.9241573 0.9074074 0.9353933 1 0.9550562 Z" />
        </clipPath>
        <clipPath id="meter-clip-left" clipPathUnits="objectBoundingBox">
          <path d="M 0 0.0449438 C 0.0925926 0.0646067 0.1851852 0.0758427 0.3888889 0.0786517 C 0.7314815 0.0814607 1 0.1516854 1 0.247191 L 1 0.752809 C 1 0.8483146 0.7314815 0.9185393 0.3888889 0.9213483 C 0.1851852 0.9241573 0.0925926 0.9353933 0 0.9550562 Z" />
        </clipPath>
      </defs>
    </svg>
  )
}

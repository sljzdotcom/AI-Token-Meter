import type { ProviderId, UsageSnapshot } from "../state/usage"
import { UsageRing } from "./UsageRing"
import { defaultStripPreferences, type StripPreferences } from "../state/stripPreferences"
import type { CSSProperties } from "react"
import { useId } from "react"
import { t, useLocale } from "../localization"

type FloatingStripProps = {
  snapshots: UsageSnapshot[]
  activeProvider: ProviderId | null
  onProviderActivate: (provider: ProviderId) => void
  preferences?: StripPreferences
  folded?: boolean
  historyNeedsAction?: boolean
  onInteraction?: (kind: "pointer" | "focus", active: boolean) => void
  onContextMenu?: () => void
}

export function FloatingStrip({ snapshots, activeProvider, onProviderActivate, preferences = defaultStripPreferences,
  folded = false, historyNeedsAction = false, onInteraction, onContextMenu }: FloatingStripProps) {
  useLocale()
  const clipId = `strip-${useId().replace(/[^a-zA-Z0-9]/g, "")}`
  const visible = preferences.orderedProviders.filter(id => !preferences.hiddenProviders.includes(id))
    .map(id => snapshots.find(s => s.providerId === id)).filter(s => s != null)
  const compact = preferences.density === "compact"
  const width = compact ? 78 : 108
  const height = (compact ? 286 : 356) - (3 - Math.max(visible.length, 1)) * (compact ? 58 : 72)
  const style = { clipPath: `url("#${clipId}-right")`, "--strip-width": `${width}px`, "--strip-height": `${height}px`,
    "--strip-ring": compact ? "48px" : "60px", "--strip-gap": compact ? "10px" : "12px",
    "--strip-logo": compact ? "21px" : "26.4px", "--strip-line": compact ? "3.5px" : "5px" } as CSSProperties
  if (folded) return <button aria-label={t("Expand floating meter")} className="meter-folded"
    onPointerEnter={() => onInteraction?.("pointer", true)}
    onPointerLeave={() => onInteraction?.("pointer", false)}
    onFocus={() => onInteraction?.("focus", true)} onBlur={() => onInteraction?.("focus", false)}
    onContextMenu={event => { event.preventDefault(); onContextMenu?.() }}
    onClick={() => onInteraction?.("pointer", true)}><span /></button>
  return (
    <>
      <MeterClipPaths density={preferences.density} count={visible.length} idPrefix={clipId} />
      <nav
        aria-label={t("AI usage providers")}
        className="floating-strip"
        data-density={preferences.density}
        style={style}
        onPointerEnter={() => onInteraction?.("pointer", true)}
        onPointerLeave={() => onInteraction?.("pointer", false)}
        onFocus={() => onInteraction?.("focus", true)}
        onBlur={event => { if (!event.currentTarget.contains(event.relatedTarget)) onInteraction?.("focus", false) }}
        onContextMenu={event => { event.preventDefault(); onContextMenu?.() }}
        onPointerDown={(event) => {
          if (event.button === 0 && !(event.target as HTMLElement).closest("button")) {
            window.dispatchEvent(new CustomEvent("meter-drag-requested"))
          }
        }}
      >
        <div aria-hidden="true" className="floating-strip__drag-handle" />
        {visible.map((snapshot) => (
          <UsageRing
            key={snapshot.providerId}
            onActivate={() => onProviderActivate(snapshot.providerId)}
            selected={activeProvider === snapshot.providerId}
            needsAction={snapshot.providerId === "deepseek" && historyNeedsAction}
            snapshot={snapshot}
          />
        ))}
      </nav>
    </>
  )
}

export function MeterClipPaths({ density = "comfortable", count = 3, idPrefix = "strip-clip" }: { density?: string, count?: number, idPrefix?: string }) {
  const compact = density === "compact"
  const width = compact ? 78 : 108
  const removed = (3 - Math.max(count, 1)) * (compact ? 58 : 72)
  const height = (compact ? 286 : 356) - removed
  const path = compact
    ? `M 78 12 C 71 17 63 21 48 22 C 21 23 0 42 0 70 L 0 ${216-removed} C 0 ${244-removed} 21 ${263-removed} 48 ${264-removed} C 63 ${265-removed} 71 ${269-removed} 78 ${274-removed} Z`
    : `M 108 16 C 98 23 88 27 66 28 C 29 29 0 54 0 88 L 0 ${268-removed} C 0 ${302-removed} 29 ${327-removed} 66 ${328-removed} C 88 ${329-removed} 98 ${333-removed} 108 ${340-removed} Z`
  return (
    <svg aria-hidden="true" className="meter-clip-paths" focusable="false">
      <defs>
        <clipPath id={`${idPrefix}-right`} clipPathUnits="objectBoundingBox">
          <path d={path} transform={`scale(${1/width} ${1/height})`} />
        </clipPath>
        <clipPath id={`${idPrefix}-left`} clipPathUnits="objectBoundingBox">
          <path d={path} transform={`translate(1 0) scale(${-1/width} ${1/height})`} />
        </clipPath>
      </defs>
    </svg>
  )
}

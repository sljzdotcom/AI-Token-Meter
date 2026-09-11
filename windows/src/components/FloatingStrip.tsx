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
  const comfortable = preferences.density === "comfortable"
  const width = preferences.density === "mini" ? 65 : preferences.density === "compact" ? 78 : 108
  const contentHeight = (comfortable ? 356 : 286) - (3 - Math.max(visible.length, 1)) * (comfortable ? 72 : 58)
  const height = contentHeight
  const style = { clipPath: `url("#${clipId}-right")`, "--strip-width": `${width}px`, "--strip-height": `${height}px`,
    "--strip-content-height": `${contentHeight}px`,
    "--strip-ring": comfortable ? "60px" : "48px", "--strip-gap": comfortable ? "12px" : "10px",
    "--strip-logo": comfortable ? "26.4px" : "21px", "--strip-line": comfortable ? "5px" : "3.5px" } as CSSProperties
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
        <div className="floating-strip__providers">
          {visible.map((snapshot) => (
            <UsageRing
              key={snapshot.providerId}
              onActivate={() => onProviderActivate(snapshot.providerId)}
              selected={activeProvider === snapshot.providerId}
              needsAction={snapshot.providerId === "deepseek" && historyNeedsAction}
              snapshot={snapshot}
            />
          ))}
        </div>
      </nav>
    </>
  )
}

export function MeterClipPaths({ density = "comfortable", count = 3, idPrefix = "strip-clip" }: { density?: string, count?: number, idPrefix?: string }) {
  const comfortable = density === "comfortable"
  const width = density === "mini" ? 65 : density === "compact" ? 78 : 108
  const removed = (3 - Math.max(count, 1)) * (comfortable ? 72 : 58)
  const contentHeight = (comfortable ? 356 : 286) - removed
  const height = contentHeight
  const path = density === "mini"
    ? `M 65 8 C 59 14 53 21 42 22 C 18 23 0 42 0 70 L 0 ${height-70} C 0 ${height-42} 18 ${height-23} 42 ${height-22} C 53 ${height-21} 59 ${height-14} 65 ${height-8} Z`
    : density === "compact"
      ? `M 78 8 C 71 14 63 21 48 22 C 21 23 0 42 0 70 L 0 ${height-70} C 0 ${height-42} 21 ${height-23} 48 ${height-22} C 63 ${height-21} 71 ${height-14} 78 ${height-8} Z`
      : `M 108 16 C 98 23 88 27 66 28 C 29 29 0 54 0 88 L 0 ${height-88} C 0 ${height-54} 29 ${height-29} 66 ${height-28} C 88 ${height-27} 98 ${height-23} 108 ${height-16} Z`
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

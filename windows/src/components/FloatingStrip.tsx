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
  onSettingsOpen?: () => void
}

export function FloatingStrip({ snapshots, activeProvider, onProviderActivate, preferences = defaultStripPreferences,
  folded = false, historyNeedsAction = false, onInteraction, onContextMenu, onSettingsOpen }: FloatingStripProps) {
  useLocale()
  const clipId = `strip-${useId().replace(/[^a-zA-Z0-9]/g, "")}`
  const visible = preferences.orderedProviders.filter(id => !preferences.hiddenProviders.includes(id))
    .map(id => snapshots.find(s => s.providerId === id)).filter(s => s != null)
  const compact = preferences.density === "compact"
  const width = compact ? 65 : 108
  const contentHeight = (compact ? 286 : 356) - (3 - Math.max(visible.length, 1)) * (compact ? 58 : 72)
  const height = contentHeight + (compact ? 42 : 48)
  const style = { clipPath: `url("#${clipId}-right")`, "--strip-width": `${width}px`, "--strip-height": `${height}px`,
    "--strip-content-height": `${contentHeight}px`,
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
        <button type="button" className="floating-strip__settings" aria-label={t("Settings")}
          onClick={() => onSettingsOpen?.()} title={t("Settings")}>⚙</button>
      </nav>
    </>
  )
}

export function MeterClipPaths({ density = "comfortable", count = 3, idPrefix = "strip-clip" }: { density?: string, count?: number, idPrefix?: string }) {
  const compact = density === "compact"
  const width = compact ? 65 : 108
  const removed = (3 - Math.max(count, 1)) * (compact ? 58 : 72)
  const contentHeight = (compact ? 286 : 356) - removed
  const height = contentHeight + (compact ? 42 : 48)
  const path = compact
    ? `M 65 8 C 59 14 53 21 42 22 C 18 23 0 42 0 70 L 0 ${contentHeight-70} C 0 ${contentHeight-36} 6 ${contentHeight-15} 18 ${contentHeight-10} C 27 ${contentHeight-8} 34 ${contentHeight+1} 36 ${contentHeight+8} C 40 ${contentHeight+13} 45 ${height-30} 48 ${height-24} C 52 ${height-15} 59 ${height-9} 65 ${height-6} Z`
    : `M 108 16 C 98 23 88 27 66 28 C 29 29 0 54 0 88 L 0 ${contentHeight-88} C 0 ${contentHeight-47} 13 ${contentHeight-12} 34 ${contentHeight-10} C 50 ${contentHeight-8} 64 ${contentHeight+3} 68 ${contentHeight+12} C 78 ${contentHeight+20} 96 ${height-14} 108 ${height-8} Z`
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

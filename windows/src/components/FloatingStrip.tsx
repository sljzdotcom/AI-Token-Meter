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
  const path = meterContourPath(density, count)
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

export function meterContourPath(density: string, count: number) {
  const comfortable = density === "comfortable"
  const width = density === "mini" ? 65 : density === "compact" ? 78 : 108
  const height = (comfortable ? 356 : 286) - (3 - Math.max(count, 1)) * (comfortable ? 72 : 58)
  const shoulderDepth = comfortable ? 88 : 70
  const widthScale = width / 65
  const depthScale = shoulderDepth / 70
  const x = (value: number) => value * widthScale
  const y = (value: number) => value * depthScale
  return `M ${x(65)} ${y(4)} C ${x(63)} ${y(18)} ${x(54)} ${y(29)} ${x(37)} ${y(30)} C ${x(18)} ${y(31)} ${x(5)} ${y(42)} ${x(1)} ${y(58)} C ${x(0)} ${y(62)} ${x(0)} ${y(66)} ${x(0)} ${y(70)} L 0 ${height-shoulderDepth} C ${x(0)} ${height-y(66)} ${x(0)} ${height-y(62)} ${x(1)} ${height-y(58)} C ${x(5)} ${height-y(42)} ${x(18)} ${height-y(31)} ${x(37)} ${height-y(30)} C ${x(54)} ${height-y(29)} ${x(63)} ${height-y(18)} ${x(65)} ${height-y(4)} Z`
}

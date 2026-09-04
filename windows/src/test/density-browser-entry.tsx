import { flushSync } from "react-dom"
import { createRoot } from "react-dom/client"
import type { CSSProperties } from "react"

import { FloatingStrip } from "../components/FloatingStrip"
import { ProviderDetail } from "../details/ProviderDetail"
import { SettingsWindow } from "../settings/SettingsWindow"
import type { UsageSnapshot } from "../state/usage"
import "../styles.css"

const displayStyle = {
  "--display-font": "Antonio, 'Segoe UI Variable', sans-serif",
} as CSSProperties

const snapshot: UsageSnapshot = {
  schemaVersion: 1,
  providerId: "claude",
  displayName: "Claude Code",
  status: "fresh",
  usedRatio: 0.23,
  primaryMetric: { label: "Session", current: 23, limit: 100, unit: "percent", kind: "officialLimit" },
  fetchedAt: "2026-09-04T00:00:00Z",
  staleAfterSeconds: 300,
}

const root = document.getElementById("root")!
root.style.fontFamily = "Antonio, 'Segoe UI Variable', sans-serif"

flushSync(() => {
  createRoot(root).render(
    <>
      <main className="meter-stage" style={displayStyle}>
        <FloatingStrip activeProvider={null} onProviderActivate={() => {}} snapshots={[snapshot]} />
      </main>
      <main className="detail-surface" style={displayStyle}>
        <ProviderDetail
          onInteractionEnd={() => {}}
          onInteractionStart={() => {}}
          onPointerEnter={() => {}}
          onPointerLeave={() => {}}
          snapshot={snapshot}
        />
      </main>
      <SettingsWindow displayFont="Antonio" onDisplayFontChange={() => {}} />
    </>,
  )
})

function styleFor<T extends Element>(selector: string): CSSStyleDeclaration {
  const element = document.querySelector<T>(selector)
  if (!element) throw new Error(`Missing ${selector}`)
  return getComputedStyle(element)
}

requestAnimationFrame(() => {
  const report = {
    meterFont: styleFor(".meter-stage").fontFamily,
    detailFont: styleFor(".provider-detail").fontFamily,
    detailBody: styleFor(".provider-detail").fontSize,
    identityTitle: styleFor(".provider-detail__identity strong").fontSize,
    headline: styleFor(".provider-detail__headline").fontSize,
    sectionTitle: styleFor(".detail-section h2").fontSize,
    cardNumber: styleFor(".metric-card strong").fontSize,
    settingsFont: styleFor(".settings-window").fontFamily,
    settingsBase: styleFor(".settings-window").fontSize,
    settingsTitle: styleFor(".settings-window > header strong").fontSize,
    controlFont: styleFor<HTMLSelectElement>("select[aria-label='Display font']").fontSize,
    controlMinHeight: styleFor<HTMLSelectElement>("select[aria-label='Display font']").minHeight,
    colorScheme: styleFor(".settings-window").colorScheme,
    selectColor: styleFor<HTMLSelectElement>("select[aria-label='Display font']").color,
    selectBackground: styleFor<HTMLSelectElement>("select[aria-label='Display font']").backgroundColor,
    optionColor: styleFor<HTMLOptionElement>("select[aria-label='Display font'] option").color,
    optionBackground: styleFor<HTMLOptionElement>("select[aria-label='Display font'] option").backgroundColor,
  }
  document.getElementById("density-report")!.textContent = JSON.stringify(report)
})

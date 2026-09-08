import { flushSync } from "react-dom"
import { createRoot } from "react-dom/client"
import type { CSSProperties } from "react"

import { FloatingStrip } from "../components/FloatingStrip"
import { ProviderDetail } from "../details/ProviderDetail"
import { SettingsWindow, type UpdateState } from "../settings/SettingsWindow"
import type { UsageSnapshot } from "../state/usage"
import { defaultStripPreferences } from "../state/stripPreferences"
import "../styles.css"
import { setLocale } from "../localization"
import { displayFontStack } from "../displayFonts"

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
      {new URLSearchParams(location.search).has("comparison") && <aside style={{background: "#172131", padding: 24, height: 510, color: "#fff", fontFamily: "sans-serif"}}>
        <h2 style={{fontSize: 20}}>AI Token Meter · Compact / Comfortable</h2>
        <p style={{fontSize: 12, opacity: 0.6}}>Browser render · demo data · both screen edges</p>
        <div style={{display: "flex", gap: 32}}>
          {(["compact", "comfortable"] as const).flatMap(density => (["left", "right"] as const).map(edge => <div key={`${density}-${edge}`}>
            <p style={{fontSize: 12}}>{density} · {edge}</p>
            <div className={`meter-stage--strip-only meter-edge--${edge}`} style={{width: density === "compact" ? 78 : 108, height: density === "compact" ? 286 : 356}}>
              <FloatingStrip activeProvider={null} onProviderActivate={() => {}}
                preferences={{...defaultStripPreferences, density}}
                snapshots={(["claude", "codex", "deepseek"] as const).map(providerId => ({...snapshot, providerId, usedRatio: 0.25}))} />
            </div>
          </div>))}
        </div>
      </aside>}
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
      <SettingsWindow displayFont="Antonio" onDisplayFontChange={() => {}}
        onOpenAuthorLink={target => { document.getElementById("about-activation")!.textContent = target }} />
    </>,
  )
})

function styleFor<T extends Element>(selector: string): CSSStyleDeclaration {
  const element = document.querySelector<T>(selector)
  if (!element) throw new Error(`Missing ${selector}`)
  return getComputedStyle(element)
}

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
const detailSamples: Array<{scenario: string; text: string; size: number; baseline: number}> = []
for (const locale of ["en", "zh-CN"] as const) {
  flushSync(() => setLocale(locale))
  for (const font of ["Antonio", "Microsoft YaHei", "SimHei", "KaiTi"]) {
    for (const providerId of ["claude", "codex", "deepseek"] as const) {
      for (const status of ["fresh", "unavailable"] as const) {
        const host = document.createElement("div")
        host.style.cssText = "position:absolute;left:-10000px;width:440px;height:760px"
        host.style.setProperty("--display-font", displayFontStack(font))
        document.body.append(host)
        const sampleRoot = createRoot(host)
        const value: UsageSnapshot = {...snapshot, providerId, displayName: providerId, status,
          usedRatio: status === "fresh" ? .23 : null, primaryMetric: status === "fresh" ? snapshot.primaryMetric : null,
          localActivity: providerId === "claude" ? {periodDays:30,sessions:12,tokens:34567,activeDays:7} : null,
          resetCredits: providerId === "codex" ? [{kind:"fullUsageReset",count:1,expiresAt:"2026-10-01T00:00:00Z"}] : [],
          dailyHistory: providerId === "deepseek" && status === "fresh" ? [{date:"2026-09-01",costCny:1.25,tokens:3000,requests:7}] : []}
        flushSync(() => sampleRoot.render(<ProviderDetail snapshot={value} onPointerEnter={() => {}} onPointerLeave={() => {}} onInteractionStart={() => {}} onInteractionEnd={() => {}} onDeepSeekHistorySync={() => {}} deepseekHistoryStatus="failed" />))
        for (const node of host.querySelectorAll<HTMLElement>(".provider-detail, .provider-detail *")) {
          const text = [...node.childNodes].filter(child => child.nodeType === Node.TEXT_NODE).map(child => child.textContent?.trim()).filter(Boolean).join(" ")
          if (!text) continue
          const baseline = node.matches(".provider-detail__headline") ? 24 : node.matches(".provider-detail__identity strong") ? 20
            : node.matches(".metric-card strong, .reset-credit strong, .activity-grid strong, .history-summary strong") ? 18
            : node.matches("h2, footer") ? 13 : node.matches("small") ? 14 * 5 / 6 : 14
          detailSamples.push({scenario:`${locale}/${font}/${providerId}/${status}`, text, size:parseFloat(getComputedStyle(node).fontSize), baseline})
        }
        flushSync(() => sampleRoot.unmount())
        host.remove()
      }
    }
  }
}
const updateStates: UpdateState[] = [
  {phase: "idle", currentVersion: "0.5.0"},
  {phase: "checking", currentVersion: "0.5.0"},
  {phase: "upToDate", currentVersion: "0.5.0"},
  {phase: "available", currentVersion: "0.5.0", availableVersion: "0.6.0"},
  {phase: "downloading", currentVersion: "0.5.0", progressPercent: 25},
  {phase: "installing", currentVersion: "0.5.0"},
  {phase: "failed", currentVersion: "0.5.0", message: "Update check failed."},
]
const updateSamples: Array<{locale: string; phase: UpdateState["phase"]; color: string; fontFamily: string; fontWeight: string; fontSize: string}> = []
const aboutSamples: Array<{locale: string; width: number; labels: string[]; hrefs: Array<string | null>; groupName: string | null; authorVisible: boolean; headingVisible: boolean; rows: number; equalHeights: boolean; unclipped: boolean; iconSizes: Array<[number, number]>}> = []
for (const locale of ["en", "zh-CN"] as const) {
  flushSync(() => setLocale(locale))
  for (const updateState of updateStates) {
    const host = document.createElement("div")
    host.style.cssText = "position:absolute;left:-10000px;width:720px;height:640px"
    document.body.append(host)
    const sampleRoot = createRoot(host)
    flushSync(() => sampleRoot.render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="About" updateState={updateState} />))
    flushSync(() => (host.querySelectorAll<HTMLElement>('[role="tab"]')[3]).click())
    const style = getComputedStyle(host.querySelector<HTMLElement>(".update-status")!)
    updateSamples.push({locale, phase: updateState.phase, color: style.color, fontFamily: style.fontFamily, fontWeight: style.fontWeight, fontSize: style.fontSize})
    flushSync(() => sampleRoot.unmount())
    host.remove()
  }
  for (const width of [360, 720]) {
    const host = document.createElement("div")
    host.style.cssText = `position:absolute;left:-10000px;width:${width}px;height:640px`
    document.body.append(host)
    const sampleRoot = createRoot(host)
    flushSync(() => sampleRoot.render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="About" />))
    flushSync(() => host.querySelectorAll<HTMLElement>('[role="tab"]')[3].click())
    host.querySelector<HTMLElement>(".settings-window")!.style.width = `${width}px`
    const group = host.querySelector<HTMLElement>(".author-links")!
    const links = [...group.querySelectorAll<HTMLAnchorElement>("a")]
    const linkRects = links.map(link => link.getBoundingClientRect())
    const groupRect = group.getBoundingClientRect()
    aboutSamples.push({
      locale,
      width,
      labels: links.map(link => link.textContent?.trim() ?? ""),
      hrefs: links.map(link => link.getAttribute("href")),
      groupName: group.getAttribute("aria-label"),
      authorVisible: (host.textContent ?? "").includes(locale === "en" ? "Author · Miller" : "作者 · Miller"),
      headingVisible: [...group.children].some(child => child.tagName === "SMALL"),
      rows: new Set(linkRects.map(rect => Math.round(rect.top))).size,
      equalHeights: new Set(linkRects.map(rect => Math.round(rect.height))).size === 1,
      unclipped: linkRects.every(rect => rect.left >= groupRect.left && rect.right <= groupRect.right && rect.bottom <= groupRect.bottom),
      iconSizes: links.map(link => {
        const rect = link.querySelector("svg")!.getBoundingClientRect()
        return [Math.round(rect.width), Math.round(rect.height)]
      }),
    })
    flushSync(() => sampleRoot.unmount())
    host.remove()
  }
}
document.getElementById("density-report")!.textContent = JSON.stringify({...report, detailSamples, updateSamples, aboutSamples})

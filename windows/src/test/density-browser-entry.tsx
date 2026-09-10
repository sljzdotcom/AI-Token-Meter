import geminiFresh from "../../../contracts/fixtures/gemini-fresh.json"
import { flushSync } from "react-dom"
import { createRoot } from "react-dom/client"
import type { CSSProperties } from "react"

import { FloatingStrip } from "../components/FloatingStrip"
import { ProviderDetail } from "../details/ProviderDetail"
import { SettingsWindow, type UpdateState } from "../settings/SettingsWindow"
import { AuthorLinks } from "../settings/AuthorLinks"
import { unavailableSnapshots, type UsageSnapshot } from "../state/usage"
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
      {new URLSearchParams(location.search).has("comparison") && <aside style={{background: "#172131", padding: 24, height: 590, color: "#fff", fontFamily: "sans-serif"}}>
        <h2 style={{fontSize: 20}}>AI Token Meter · Compact / Comfortable</h2>
        <p style={{fontSize: 12, opacity: 0.6}}>Browser render · demo data · both screen edges</p>
        <div style={{display: "flex", gap: 32}}>
          {(["compact", "comfortable"] as const).flatMap(density => (["left", "right"] as const).map(edge => <div key={`${density}-${edge}`}>
            <p style={{fontSize: 12}}>{density} · {edge}</p>
            <div className={`meter-stage--strip-only meter-edge--${edge}`} style={{width: density === "compact" ? 56.5 : 108, height: density === "compact" ? 344 : 428}}>
              <FloatingStrip activeProvider={null} onProviderActivate={() => {}}
                preferences={{...defaultStripPreferences, density}}
                snapshots={unavailableSnapshots} />
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
const detailSurfaceSamples: Array<{providerId: string; status: string; backgroundImage: string; accent: string}> = []
for (const providerId of ["claude", "codex", "deepseek", "gemini"] as const) {
  const statuses = providerId === "gemini" ? ["fresh", "cached", "unavailable"] as const : ["fresh", "unavailable"] as const
  for (const status of statuses) {
    const host = document.createElement("div")
    host.className = "detail-surface"
    host.style.cssText = "position:absolute;left:-10000px;width:440px;height:760px"
    document.body.append(host)
    const sampleRoot = createRoot(host)
    const hasQuota = status === "fresh" || status === "cached"
    const value: UsageSnapshot = providerId === "gemini"
      ? {...geminiFresh as UsageSnapshot, providerId, status,
        primaryMetric:hasQuota ? geminiFresh.primaryMetric as UsageSnapshot["primaryMetric"] : null,
        geminiQuotaMetrics:hasQuota ? geminiFresh.geminiQuotaMetrics as UsageSnapshot["geminiQuotaMetrics"] : []}
      : {...snapshot, providerId, displayName:providerId, status,
        usedRatio:hasQuota ? .23 : null, primaryMetric:hasQuota ? snapshot.primaryMetric : null}
    flushSync(() => sampleRoot.render(<ProviderDetail snapshot={value} onPointerEnter={() => {}} onPointerLeave={() => {}} onInteractionStart={() => {}} onInteractionEnd={() => {}} />))
    const detail = host.querySelector<HTMLElement>(".provider-detail")!
    const style = getComputedStyle(detail)
    detailSurfaceSamples.push({
      providerId,
      status,
      backgroundImage: style.backgroundImage,
      accent: style.getPropertyValue("--detail-accent").trim(),
    })
    flushSync(() => sampleRoot.unmount())
    host.remove()
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
const aboutCopySamples: Array<{locale: string; authorVisible: boolean; headingVisible: boolean; labels: string[]; hrefs: Array<string | null>; linkCount: number}> = []
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
  const settingsHost = document.createElement("div")
  settingsHost.style.cssText = "position:absolute;left:-10000px;width:760px;height:560px"
  document.body.append(settingsHost)
  const settingsRoot = createRoot(settingsHost)
  flushSync(() => settingsRoot.render(<SettingsWindow displayFont="System Default" onDisplayFontChange={() => {}} requestedTab="About" />))
  flushSync(() => settingsHost.querySelectorAll<HTMLElement>('[role="tab"]')[3].click())
  const settingsGroup = settingsHost.querySelector<HTMLElement>(".author-links")!
  const settingsLinks = [...settingsGroup.querySelectorAll<HTMLAnchorElement>("a")]
  aboutCopySamples.push({
    locale,
    authorVisible: (settingsHost.textContent ?? "").includes(locale === "en" ? "Author · Miller" : "作者 · Miller"),
    headingVisible: [...settingsGroup.children].some(child => child.tagName === "SMALL"),
    labels: settingsLinks.map(link => link.textContent?.trim() ?? ""),
    hrefs: settingsLinks.map(link => link.getAttribute("href")),
    linkCount: settingsLinks.length,
  })
  flushSync(() => settingsRoot.unmount())
  settingsHost.remove()
  for (const width of [240, 760]) {
    const host = document.createElement("div")
    host.style.cssText = `position:absolute;left:-10000px;width:${width}px`
    document.body.append(host)
    const sampleRoot = createRoot(host)
    flushSync(() => sampleRoot.render(<AuthorLinks onOpen={() => {}} />))
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
      authorVisible: false,
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

// Exercise real CSS clipping and hit testing. Literal sizes come from the approved
// 1–4-row specification, independent of FloatingStrip's size calculation.
const stripSamples = []
flushSync(() => setLocale("en"))
for (const density of ["compact", "comfortable"] as const) {
  for (const edge of ["left", "right"] as const) {
    for (const count of [1, 2, 3, 4]) {
      const width = density === "compact" ? 56.5 : 108
      const height = (density === "compact" ? [170,228,286,344] : [212,284,356,428])[count-1]
      const host = document.createElement("div")
      host.className = `meter-stage--strip-only meter-edge--${edge}`
      host.style.cssText = `position:fixed;left:20px;top:20px;width:${width}px;height:${height}px;z-index:2147483647`
      document.body.appendChild(host)
      const sampleRoot = createRoot(host)
      const activated: string[] = []
      flushSync(() => sampleRoot.render(<FloatingStrip activeProvider={null} onProviderActivate={id => activated.push(id)}
        preferences={{...defaultStripPreferences, density, hiddenProviders: defaultStripPreferences.orderedProviders.slice(count)}} snapshots={unavailableSnapshots} />))
      const nav = host.querySelector<HTMLElement>("nav")!
      const buttons = [...host.querySelectorAll<HTMLButtonElement>("button")]
      for (const button of buttons) button.style.animation = "none"
      const rect = nav.getBoundingClientRect()
      const hitButtons = buttons.every(button => {
        const bounds = button.getBoundingClientRect()
        const target = document.elementFromPoint(bounds.x + bounds.width/2, bounds.y + bounds.height/2)
        return target != null && button.contains(target) && bounds.top >= rect.top && bounds.bottom <= rect.bottom
      })
      let drags = 0
      const drag = () => { drags += 1 }
      window.addEventListener("meter-drag-requested", drag)
      nav.dispatchEvent(new PointerEvent("pointerdown", {bubbles:true,button:0}))
      for (const button of buttons) {
        button.dispatchEvent(new PointerEvent("pointerdown", {bubbles:true,button:0}))
        button.click()
      }
      window.removeEventListener("meter-drag-requested", drag)
      const firstButtonWidth = buttons[0]?.getBoundingClientRect().width ?? 0
      stripSamples.push({density,edge,count,width:rect.width,height:rect.height,expectedWidth:width,expectedHeight:height,
        sideMargin:(rect.width-firstButtonWidth)/2,expectedSideMargin:density === "compact" ? 4.25 : 24,
        hitButtons,drags,activated,expectedOrder:defaultStripPreferences.orderedProviders.slice(0,count),
        buttonCount:buttons.length,geminiProgress:host.querySelector('[aria-label="Gemini usage"][role="progressbar"]')?.getAttribute("aria-valuenow") ?? null,
        mirroredLogo: buttons.some(button => getComputedStyle(button.querySelector("svg")!).transform !== "none")})
      flushSync(() => sampleRoot.unmount())
      host.remove()
    }
  }
}
const geminiSamples = []
for (const width of [340, 440]) {
  for (const status of ["fresh", "cached", "authenticationRequired", "unavailable"] as const) {
    const hasQuota = status === "fresh" || status === "cached"
    const host = document.createElement("div")
    host.className = "detail-surface"
    host.style.cssText = `position:fixed;left:0;top:0;width:${width}px;height:760px;z-index:2147483647`
    document.body.append(host)
    const sampleRoot = createRoot(host)
    let retries = 0, guides = 0
    const value: UsageSnapshot = {...geminiFresh, status, providerId:"gemini", fetchedAt:new Date().toISOString(),
      usedRatio:hasQuota ? .6 : null, primaryMetric:hasQuota ? geminiFresh.primaryMetric as UsageSnapshot["primaryMetric"] : null,
      secondaryMetric:null, geminiQuotaMetrics:hasQuota ? geminiFresh.geminiQuotaMetrics as NonNullable<UsageSnapshot["geminiQuotaMetrics"]> : [],
      statusMessage:status === "cached" ? "Cached · sign in required" : status === "unavailable" ? "Antigravity CLI configuration is not supported" : null}
    flushSync(() => sampleRoot.render(<ProviderDetail snapshot={value} onPointerEnter={()=>{}} onPointerLeave={()=>{}} onInteractionStart={()=>{}} onInteractionEnd={()=>{}} onCheckGeminiStatus={()=>{retries++}} onOpenGeminiInstallationGuide={()=>{guides++}} />))
    const cards = [...host.querySelectorAll<HTMLElement>(".metric-card")]
    const texts = cards.map(card=>card.textContent ?? "")
    const bounds = host.getBoundingClientRect()
    const nodes = [...host.querySelectorAll<HTMLElement>(".metric-card, .service-actions button, footer")]
    for (const button of host.querySelectorAll<HTMLButtonElement>("button")) button.click()
    const clipped = nodes.flatMap(node=>{const r=node.getBoundingClientRect();return r.left>=bounds.left && r.right<=bounds.right && r.top>=bounds.top && r.bottom<=bounds.bottom ? [] : [{className:node.className,top:r.top,bottom:r.bottom,left:r.left,right:r.right}]})
    geminiSamples.push({width,status,hasQuota,texts,retries,guides,reasonVisible:status !== "cached" || host.textContent!.includes("Cached · sign in required"),
      unclipped:clipped.length===0,clipped})
    flushSync(()=>sampleRoot.unmount());host.remove()
  }
}

document.getElementById("density-report")!.textContent = JSON.stringify({...report, detailSamples, detailSurfaceSamples, updateSamples, aboutSamples, aboutCopySamples, stripSamples, geminiSamples})

import { existsSync } from "node:fs"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

import {
  extractPreviewUrl,
  runBrowserCandidates,
  runWithCleanup,
  selectBrowserCandidates,
  waitForHttpReady,
  spawnDensityPreview,
  stopProcessTree,
} from "./density-process-lifecycle.mjs"

const windowsRoot = resolve(fileURLToPath(new URL("..", import.meta.url)))
const browsers = findBrowsers()
const vite = startPreview()

const started = Date.now()
await runWithCleanup(async () => {
  const baseUrl = await vite.ready
  await waitForHttpReady(`${baseUrl}density-browser.html`)
  console.log(`Preview ready after ${Date.now() - started}ms; verifying browser density styles with ${browsers.map(browser => browser.label).join(", ")}`)
  const result = await runBrowserCandidates(browsers, `${baseUrl}density-browser.html`, {
    onDiagnostic: message => console.log(`[density] ${message}`),
  })
  const report = densityReport(result.output)
  assertDensity(report)
  if (report.stripSamples?.length !== 24) throw new Error("Missing strip geometry scenarios")
  for (const sample of report.stripSamples) {
    if (sample.width !== sample.expectedWidth || sample.height !== sample.expectedHeight
      || Math.abs(sample.sideMargin - sample.expectedSideMargin) > 0.01 || !sample.hitButtons || !sample.ringPerimetersVisible || !sample.settingsVisible
      || sample.settingsOpacityAtRest !== "0" || sample.settingsOpacityAtFocus !== "1"
      || sample.settingsHoveredAfterBodyEntry !== "false" || sample.settingsHoveredAfterZoneEntry !== "true"
      || sample.buttonCount !== sample.count || sample.drags !== 1 || sample.mirroredLogo
      || JSON.stringify(sample.activated) !== JSON.stringify(sample.expectedOrder) || sample.geminiProgress !== null) {
      throw new Error(`Strip geometry/interaction mismatch: ${JSON.stringify(sample)}`)
    }
  }
  if (report.geminiSamples?.length !== 8) throw new Error("Missing Antigravity detail states")
  for (const sample of report.geminiSamples) {
    if (!sample.unclipped || sample.retries !== 1 || sample.guides !== 1 || !sample.reasonVisible
      || (sample.hasQuota ? sample.texts.length !== 4
        || !sample.texts.some(text=>text.includes("Gemini · Five hour") && text.includes("40% remaining"))
        || !sample.texts.some(text=>text.includes("Claude/GPT · Weekly") && text.includes("80% remaining"))
        : sample.texts.some(text=>text.includes("0% remaining")))) {
      throw new Error(`Antigravity detail state mismatch: ${JSON.stringify(sample)}`)
    }
    if (sample.accentRoles?.title !== "rgb(62, 214, 178)") {
      throw new Error(`Antigravity title lost its approved accent: ${JSON.stringify(sample.accentRoles)}`)
    }
    if (sample.hasQuota && (
      sample.accentRoles.values.length !== sample.texts.length
      || sample.accentRoles.values.some(color => color !== "rgb(62, 214, 178)")
      || sample.accentRoles.bars.length !== sample.texts.length
      || sample.accentRoles.bars.some(background => !background.includes("linear-gradient"))
    )) {
      throw new Error(`Antigravity quota accents are incomplete: ${JSON.stringify(sample.accentRoles)}`)
    }
  }
  if (report.detailSurfaceSamples?.length !== 9) throw new Error("Missing Provider detail surface scenarios")
  for (const sample of report.detailSurfaceSamples) {
    if (!sample.backgroundImage.includes("linear-gradient")
      || !sample.backgroundImage.includes("rgb(17, 24, 38)")
      || !sample.backgroundImage.includes("rgb(7, 12, 21)")
      || (sample.providerId === "gemini" && sample.accent !== "#3ed6b2")) {
      throw new Error(`Provider detail surface mismatch: ${JSON.stringify(sample)}`)
    }
  }
  console.log("Antigravity detail verified: 8 fresh/cache/auth/unavailable clipping and action scenarios")
  console.log("Four-provider strip geometry verified: 24 real CSS clipping/hit-test scenarios")
  console.log(`Browser density styles verified with ${result.browser.label}: ${report.detailSamples.length} text roles across providers, locales and fonts`)
}, async () => {
  await stopVite(vite.process)
  console.log(`[density] preview cleanup complete; total ${Date.now() - started}ms`)
})

function findBrowsers() {
  const programFiles = process.env.ProgramFiles ?? "C:\\Program Files"
  const programFilesX86 = process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)"
  const localAppData = process.env.LOCALAPPDATA ?? ""
  const candidates = [
    ["BROWSER_BIN", process.env.BROWSER_BIN],
    ["CHROME_BIN", process.env.CHROME_BIN],
    ["Google Chrome (macOS)", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"],
    ["Microsoft Edge (macOS)", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"],
    ["Microsoft Edge (ProgramFiles)", join(programFiles, "Microsoft", "Edge", "Application", "msedge.exe")],
    ["Microsoft Edge (ProgramFiles x86)", join(programFilesX86, "Microsoft", "Edge", "Application", "msedge.exe")],
    ["Microsoft Edge (LOCALAPPDATA)", localAppData && join(localAppData, "Microsoft", "Edge", "Application", "msedge.exe")],
    ["Google Chrome (ProgramFiles)", join(programFiles, "Google", "Chrome", "Application", "chrome.exe")],
    ["Google Chrome (ProgramFiles x86)", join(programFilesX86, "Google", "Chrome", "Application", "chrome.exe")],
    ["Google Chrome (Linux)", "/usr/bin/google-chrome"],
    ["Chromium (Linux)", "/usr/bin/chromium"],
  ].filter(([, path]) => path)
  const browsers = selectBrowserCandidates(candidates, {
    exists: existsSync,
  })
  if (!browsers.length) {
    const checked = candidates.map(([label, path]) => `${label}: ${path}`).join("\n")
    throw new Error(`No supported Chrome or Edge executable was found. Checked:\n${checked}\nSet BROWSER_BIN or CHROME_BIN to override.`)
  }
  return browsers
}

function startPreview() {
  const vite = spawnDensityPreview(windowsRoot)
  let output = ""
  const ready = new Promise((resolveUrl, reject) => {
    const timeout = setTimeout(() => reject(new Error(`Timed out starting Vite:\n${output}`)), 10_000)
    const findUrl = (chunk) => {
      output += chunk.toString()
      const url = extractPreviewUrl(output)
      if (url) {
        clearTimeout(timeout)
        resolveUrl(url)
      }
    }
    vite.stdout.on("data", findUrl)
    vite.stderr.on("data", findUrl)
    vite.once("error", (error) => {
      clearTimeout(timeout)
      reject(error)
    })
    vite.once("exit", (code) => {
      clearTimeout(timeout)
      reject(new Error(`Vite exited ${code} before becoming ready:\n${output}`))
    })
  })
  return { process: vite, ready }
}

async function stopVite(vite) {
  await stopProcessTree(vite)
}

function densityReport(output) {
  const match = output.match(/<pre id="density-report"[^>]*>([^<]+)<\/pre>/)
  if (!match) {
    const preview = output.replace(/\s+/g, " ").slice(0, 800)
    throw new Error(`Browser did not publish a density report (output length ${output.length}; preview ${JSON.stringify(preview)})`)
  }
  return JSON.parse(match[1])
}

function assertDensity(report) {
  if (report.detailSamples.length < 300) throw new Error("Missing full detail typography samples")
  for (const sample of report.detailSamples) {
    if (Math.abs(sample.size - (sample.baseline - 1)) > .001) throw new Error(`${sample.scenario} ${sample.text}: expected ${sample.baseline - 1}px, received ${sample.size}px`)
  }
  const expected = {
    detailBody: "13px",
    identityTitle: "19px",
    headline: "23px",
    sectionTitle: "12px",
    cardNumber: "17px",
    settingsBase: "14px",
    settingsTitle: "20px",
    controlFont: "13px",
    controlMinHeight: "32px",
    colorScheme: "light",
    selectColor: "rgb(21, 24, 33)",
    selectBackground: "rgb(255, 255, 255)",
    optionColor: "rgb(21, 24, 33)",
    optionBackground: "rgb(255, 255, 255)",
  }
  for (const [property, value] of Object.entries(expected)) {
    if (report[property] !== value) throw new Error(`${property}: expected ${value}, received ${report[property]}`)
  }
  if (JSON.stringify(report.stripDensityOptions) !== JSON.stringify(["comfortable", "compact", "mini"])
    || report.selectedStripDensity !== "compact"
    || report.initialAutomaticCollapse !== true
    || report.automaticCollapseAfterClick !== false
    || report.collapseDelayControlsDisabled !== true
    || JSON.stringify(report.initialCollapseDelayValues) !== JSON.stringify(["150", "800"])
    || JSON.stringify(report.collapseDelayValuesAfterClick) !== JSON.stringify(["150", "800"])) {
    throw new Error(`Floating strip settings contract changed: ${JSON.stringify({
      options: report.stripDensityOptions,
      selected: report.selectedStripDensity,
      initialAutomaticCollapse: report.initialAutomaticCollapse,
      automaticCollapseAfterClick: report.automaticCollapseAfterClick,
      collapseDelayControlsDisabled: report.collapseDelayControlsDisabled,
      initialCollapseDelayValues: report.initialCollapseDelayValues,
      collapseDelayValuesAfterClick: report.collapseDelayValuesAfterClick,
    })}`)
  }
  if (!report.meterFont.includes("Antonio") || !report.detailFont.includes("Antonio")) {
    throw new Error("Settings system font leaked into the meter or Provider detail")
  }
  if (!report.settingsFont.startsWith('"Segoe UI Variable"')) throw new Error("Settings did not retain its system font")
  if (report.updateSamples.length !== 14) throw new Error("Missing bilingual update status samples")
  for (const locale of ["en", "zh-CN"]) {
    const samples = report.updateSamples.filter(sample => sample.locale === locale)
    const available = samples.find(sample => sample.phase === "available")
    if (!available) throw new Error(`${locale} available update sample was missing`)
    if (available.color !== "rgb(153, 27, 27)") throw new Error(`${locale} available update color was ${available.color}`)
    if (available.fontWeight !== "700") throw new Error(`${locale} available update font weight was ${available.fontWeight}`)
    for (const sample of samples.filter(sample => sample.phase !== "available")) {
      if (sample.color === available.color || sample.fontWeight === available.fontWeight) {
        throw new Error(`${locale} ${sample.phase} inherited available update emphasis`)
      }
      if (sample.fontSize !== available.fontSize) throw new Error(`${locale} ${sample.phase} changed update status font size`)
      if (sample.fontFamily !== available.fontFamily) throw new Error(`${locale} ${sample.phase} changed update status font family`)
    }
  }
  const expectedLabels = ["@MillerPanYue", "GitHub", "Telegram @sljzdotcom"]
  const expectedHrefs = ["https://twitter.com/MillerPanYue", "https://github.com/sljzdotcom/AI-Token-Meter", "https://t.me/sljzdotcom"]
  if (report.aboutSamples.length !== 4) throw new Error("Missing bilingual wide and narrow About samples")
  if (report.aboutCopySamples.length !== 2
    || new Set(report.aboutCopySamples.map(sample => sample.locale)).size !== 2
    || report.aboutCopySamples.some(sample =>
    !["en", "zh-CN"].includes(sample.locale)
    || sample.authorVisible
    || sample.headingVisible
    || sample.linkCount !== 3
    || JSON.stringify(sample.labels) !== JSON.stringify(expectedLabels)
    || JSON.stringify(sample.hrefs) !== JSON.stringify(expectedHrefs)
  )) {
    throw new Error("The native-size About panel restored removed visible author copy")
  }
  for (const sample of report.aboutSamples) {
    if (JSON.stringify(sample.labels) !== JSON.stringify(expectedLabels)) throw new Error(`${sample.locale}/${sample.width} About labels changed`)
    if (JSON.stringify(sample.hrefs) !== JSON.stringify(expectedHrefs)) throw new Error(`${sample.locale}/${sample.width} About targets changed`)
    const expectedGroupName = sample.locale === "en" ? "Author links" : "作者链接"
    if (sample.groupName !== expectedGroupName) throw new Error(`${sample.locale}/${sample.width} lost its accessible group name`)
    if (sample.authorVisible || sample.headingVisible) throw new Error(`${sample.locale}/${sample.width} restored removed visible author copy`)
    if (!sample.equalHeights || !sample.unclipped) throw new Error(`${sample.locale}/${sample.width} clipped or misaligned an About link`)
    if (sample.iconSizes.some(([width, height]) => width !== 15 || height !== 15)) throw new Error(`${sample.locale}/${sample.width} changed an About icon size`)
    if (sample.width === 240 && sample.rows < 2) throw new Error(`${sample.locale} narrow About links did not wrap`)
    if (sample.width === 760 && sample.rows !== 1) throw new Error(`${sample.locale} wide About links did not stay aligned`)
  }
}

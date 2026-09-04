import { spawn } from "node:child_process"
import { existsSync } from "node:fs"
import { join, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const windowsRoot = resolve(fileURLToPath(new URL("..", import.meta.url)))
const browser = findBrowser()
const vite = startVite()

try {
  const baseUrl = await vite.ready
  await waitForVite(baseUrl)
  const output = await runBrowser(browser.path, `${baseUrl}density-browser.html`)
  const report = densityReport(output)
  assertDensity(report)
  console.log(`Browser density styles verified with ${browser.label}:`, JSON.stringify(report))
} finally {
  await stopVite(vite.process)
}

function findBrowser() {
  const programFiles = process.env.ProgramFiles ?? "C:\\Program Files"
  const programFilesX86 = process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)"
  const localAppData = process.env.LOCALAPPDATA ?? ""
  const candidates = [
    ["BROWSER_BIN", process.env.BROWSER_BIN],
    ["CHROME_BIN", process.env.CHROME_BIN],
    ["Google Chrome (macOS)", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"],
    ["Microsoft Edge (macOS)", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"],
    ["Google Chrome (ProgramFiles)", join(programFiles, "Google", "Chrome", "Application", "chrome.exe")],
    ["Google Chrome (ProgramFiles x86)", join(programFilesX86, "Google", "Chrome", "Application", "chrome.exe")],
    ["Microsoft Edge (ProgramFiles)", join(programFiles, "Microsoft", "Edge", "Application", "msedge.exe")],
    ["Microsoft Edge (ProgramFiles x86)", join(programFilesX86, "Microsoft", "Edge", "Application", "msedge.exe")],
    ["Microsoft Edge (LOCALAPPDATA)", localAppData && join(localAppData, "Microsoft", "Edge", "Application", "msedge.exe")],
    ["Google Chrome (Linux)", "/usr/bin/google-chrome"],
    ["Chromium (Linux)", "/usr/bin/chromium"],
  ].filter(([, path]) => path)
  const browser = candidates.find(([, path]) => existsSync(path))
  if (!browser) {
    const checked = candidates.map(([label, path]) => `${label}: ${path}`).join("\n")
    throw new Error(`No supported Chrome or Edge executable was found. Checked:\n${checked}\nSet BROWSER_BIN or CHROME_BIN to override.`)
  }
  return { label: browser[0], path: browser[1] }
}

function startVite() {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm"
  const vite = spawn(npm, ["run", "dev", "--", "--host", "127.0.0.1", "--port", "0"], {
    cwd: windowsRoot,
    stdio: ["ignore", "pipe", "pipe"],
  })
  let output = ""
  const ready = new Promise((resolveUrl, reject) => {
    const timeout = setTimeout(() => reject(new Error(`Timed out starting Vite:\n${output}`)), 10_000)
    const findUrl = (chunk) => {
      output += chunk.toString()
      const match = output.match(/Local:\s+(http:\/\/127\.0\.0\.1:\d+\/)/)
      if (match) {
        clearTimeout(timeout)
        resolveUrl(match[1])
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

async function waitForVite(baseUrl) {
  const url = `${baseUrl}density-browser.html`
  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      const response = await fetch(url)
      if (response.ok) return
    } catch {
      // The development server is still starting.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 100))
  }
  throw new Error("Timed out waiting for Vite")
}

function runBrowser(executable, url) {
  return new Promise((resolveOutput, reject) => {
    const chrome = spawn(executable, ["--headless=new", "--disable-gpu", "--dump-dom", "--virtual-time-budget=1000", url], {
      stdio: ["ignore", "pipe", "pipe"],
    })
    let output = ""
    let errors = ""
    chrome.stdout.on("data", (chunk) => { output += chunk })
    chrome.stderr.on("data", (chunk) => { errors += chunk })
    chrome.once("error", reject)
    chrome.once("exit", (code) => code === 0 ? resolveOutput(output) : reject(new Error(`Chrome exited ${code}: ${errors}`)))
  })
}

async function stopVite(vite) {
  if (vite.exitCode !== null || vite.signalCode !== null) return
  if (process.platform === "win32") {
    await runProcess("taskkill", ["/pid", String(vite.pid), "/T", "/F"])
  } else {
    vite.kill("SIGTERM")
  }
  if (await exited(vite, 3_000)) return
  if (process.platform !== "win32") vite.kill("SIGKILL")
  if (!await exited(vite, 3_000)) throw new Error("Vite did not exit after cleanup")
}

function runProcess(command, args) {
  return new Promise((resolveProcess, reject) => {
    const child = spawn(command, args, { stdio: "ignore" })
    child.once("error", reject)
    child.once("exit", (code) => code === 0 || code === 128 ? resolveProcess() : reject(new Error(`${command} exited ${code}`)))
  })
}

function exited(vite, timeoutMs) {
  if (vite.exitCode !== null || vite.signalCode !== null) return Promise.resolve(true)
  return new Promise((resolveExit) => {
    const timeout = setTimeout(() => resolveExit(false), timeoutMs)
    vite.once("exit", () => {
      clearTimeout(timeout)
      resolveExit(true)
    })
  })
}

function densityReport(output) {
  const match = output.match(/<pre id="density-report"[^>]*>([^<]+)<\/pre>/)
  if (!match) throw new Error("Browser did not publish a density report")
  return JSON.parse(match[1])
}

function assertDensity(report) {
  const expected = {
    detailBody: "14px",
    identityTitle: "20px",
    headline: "24px",
    sectionTitle: "13px",
    cardNumber: "18px",
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
  if (!report.meterFont.includes("Antonio") || !report.detailFont.includes("Antonio")) {
    throw new Error("Settings system font leaked into the meter or Provider detail")
  }
  if (!report.settingsFont.startsWith('"Segoe UI Variable"')) throw new Error("Settings did not retain its system font")
}

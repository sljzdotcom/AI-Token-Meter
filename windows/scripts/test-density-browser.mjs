import { spawn } from "node:child_process"
import { existsSync } from "node:fs"
import { createServer } from "node:net"
import { resolve } from "node:path"
import { fileURLToPath } from "node:url"

const windowsRoot = resolve(fileURLToPath(new URL("..", import.meta.url)))
const chrome = findChrome()
const port = await freePort()
const server = startVite(port)

try {
  await waitForVite(port)
  const output = await runChrome(chrome, `http://127.0.0.1:${port}/density-browser.html`)
  const report = densityReport(output)
  assertDensity(report)
  console.log("Browser density styles verified:", JSON.stringify(report))
} finally {
  server.kill()
}

function findChrome() {
  const candidates = [
    process.env.CHROME_BIN,
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    "/usr/bin/google-chrome",
    "/usr/bin/chromium",
  ].filter(Boolean)
  const chrome = candidates.find((candidate) => existsSync(candidate))
  if (!chrome) throw new Error("Chrome is required; set CHROME_BIN to its executable path")
  return chrome
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const probe = createServer()
    probe.once("error", reject)
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address()
      if (!address || typeof address === "string") return reject(new Error("Unable to allocate a test port"))
      probe.close((error) => error ? reject(error) : resolvePort(address.port))
    })
  })
}

function startVite(port) {
  const npm = process.platform === "win32" ? "npm.cmd" : "npm"
  return spawn(npm, ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(port)], {
    cwd: windowsRoot,
    stdio: "ignore",
  })
}

async function waitForVite(port) {
  const url = `http://127.0.0.1:${port}/density-browser.html`
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

function runChrome(executable, url) {
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
  if (!report.settingsFont.includes("Segoe UI")) throw new Error("Settings did not retain its system font")
}

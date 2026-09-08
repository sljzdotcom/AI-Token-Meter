import { spawn } from "node:child_process"
import { mkdtempSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { stripVTControlCharacters } from "node:util"

export function spawnManagedProcess(command, args, options, {
  platform = process.platform,
  spawnImpl = spawn,
} = {}) {
  return spawnImpl(command, args, {
    ...options,
    // A detached Unix child is its own process-group leader, so cleanup can
    // terminate npm together with Vite (or Chrome together with its helpers).
    detached: platform !== "win32",
  })
}

export function spawnDensityPreview(windowsRoot, {
  platform = process.platform,
  nodeExecutable = process.execPath,
  spawnImpl = spawn,
} = {}) {
  const viteEntry = join(windowsRoot, "node_modules", "vite", "bin", "vite.js")
  return spawnManagedProcess(nodeExecutable, [
    viteEntry, "preview", "--config", "vite.density.config.ts",
    "--host", "127.0.0.1", "--port", "0",
  ], {
    cwd: windowsRoot,
    stdio: ["ignore", "pipe", "pipe"],
  }, { platform, spawnImpl })
}

export function extractPreviewUrl(output) {
  const plainOutput = stripVTControlCharacters(output)
  return plainOutput.match(/Local:\s+(http:\/\/127\.0\.0\.1:\d+\/)/)?.[1] ?? null
}

export async function stopProcessTree(child, {
  platform = process.platform,
  killProcess = process.kill,
  runProcess = runBoundedCommand,
  waitForExit: wait = waitForExit,
  waitForGroupExit: waitGroup = waitForProcessGroupExit,
  timeoutMs = 3_000,
} = {}) {
  if (!child.pid) return

  if (platform === "win32") {
    try {
      await runProcess("taskkill", ["/pid", String(child.pid), "/T", "/F"], { timeoutMs })
    } catch (error) {
      if (!hasExited(child) || error.code === "PROCESS_TIMEOUT") throw error
    }
    if (!await wait(child, timeoutMs)) throw new Error("Process tree did not exit after cleanup")
    return
  }

  signalProcessGroup(child.pid, "SIGTERM", killProcess)
  // The process-group leader may exit before descendants have finished their
  // own TERM handlers. Give the entire group the grace period, rather than
  // using the leader's exit as the signal to escalate immediately.
  if (await waitGroup(child.pid, timeoutMs, { killProcess })) return
  signalProcessGroup(child.pid, "SIGKILL", killProcess)
  if (!await waitGroup(child.pid, timeoutMs, { killProcess })) {
    throw new Error("Process tree did not exit after cleanup")
  }
}

export async function runBrowser(executable, url, {
  platform = process.platform,
  // Fresh Windows profiles include cold browser startup in this wall-clock cap.
  // This is a bounded allowance, not a retry or proof of a specific CI root cause.
  timeoutMs = platform === "win32" ? 45_000 : 15_000,
  setTimer = setTimeout,
  clearTimer = clearTimeout,
  now = Date.now,
  onDiagnostic = () => {},
  spawnImpl = spawn,
  stopProcessTreeImpl = stopProcessTree,
  createBrowserProfile = createTemporaryBrowserProfile,
  removeBrowserProfile = removeTemporaryBrowserProfile,
} = {}) {
  // Windows Chrome can hand command-line requests to an already-running
  // desktop instance and then exit successfully without writing --dump-dom.
  // A disposable profile forces this verification process to own its browser.
  const profileDirectory = platform === "win32" ? createBrowserProfile() : null
  let browser
  let timeout
  let result
  let failure
  let output = ""
  let errors = ""
  let stdoutBytes = 0
  let stderrBytes = 0
  let firstStdout = null
  let firstStderr = null
  let exit = "not observed"
  let close = "not observed"
  const started = now()
  const diagnostic = (includeErrors = true) => `elapsedMs=${now() - started}; pid=${browser?.pid ?? "not spawned"}; stdoutBytes=${stdoutBytes}; stderrBytes=${stderrBytes}; firstStdoutMs=${firstStdout ?? "not observed"}; firstStderrMs=${firstStderr ?? "not observed"}; exit=${exit}; close=${close}${includeErrors ? `; stderrTail=${JSON.stringify(errors)}` : ""}`
  const report = phase => onDiagnostic(`${phase}: ${diagnostic(false)}`)

  try {
    const profileArguments = profileDirectory ? [
      "--disable-extensions",
      "--no-first-run",
      "--no-default-browser-check",
      `--user-data-dir=${profileDirectory}`,
    ] : []
    browser = spawnManagedProcess(executable, [
      "--headless=new",
      "--disable-gpu",
      ...profileArguments,
      "--dump-dom",
      // Make Chromium finish DOM capture before the outer process watchdog.
      "--timeout=10000",
      "--virtual-time-budget=1000",
      url,
    ], {
      stdio: ["ignore", "pipe", "pipe"],
    }, { platform, spawnImpl })
    report(`browser started (wall-clock budget ${timeoutMs}ms)`)
    result = await new Promise((resolveOutput, reject) => {
      browser.stdout.on("data", (chunk) => {
        stdoutBytes += Buffer.byteLength(chunk)
        firstStdout ??= now() - started
        if (stdoutBytes > 2 * 1024 * 1024) {
          reject(new Error(`Browser stdout exceeded 2 MiB: ${diagnostic()}`))
          return
        }
        output += chunk
      })
      browser.stderr.on("data", (chunk) => {
        stderrBytes += Buffer.byteLength(chunk)
        firstStderr ??= now() - started
        errors = (errors + chunk).slice(-4096)
      })
      browser.once("error", error => reject(new Error(`Browser process error: ${error.message}; ${diagnostic()}`, { cause: error })))
      browser.once("exit", (code, signal) => { exit = `${code ?? signal} at ${now() - started}ms` })
      // `exit` can precede the final stdout data on Windows. `close` is only
      // emitted after the stdio streams have closed, so the dumped DOM is
      // complete before it is parsed.
      browser.once("close", (code) => {
        close = `${code} at ${now() - started}ms`
        report("browser streams closed")
        if (code === 0) resolveOutput(output)
        else reject(new Error(`Browser exited ${code}: ${diagnostic()}`))
      })
      timeout = setTimer(() => reject(new Error(`Browser timed out after ${timeoutMs}ms: ${diagnostic()}`)), timeoutMs)
    })
  } catch (error) {
    failure = error
  } finally {
    clearTimer(timeout)
    const cleanupErrors = []
    try {
      if (browser) await stopProcessTreeImpl(browser, { platform })
      report("browser process-tree cleanup complete")
    } catch (error) { cleanupErrors.push(error) }
    try {
      if (profileDirectory) removeBrowserProfile(profileDirectory)
    } catch (error) { cleanupErrors.push(error) }
    if (cleanupErrors.length) {
      const all = [...(failure ? [failure] : []), ...cleanupErrors]
      failure = new AggregateError(all, all.map(error => error.message).join("; "))
    }
  }
  if (failure) throw failure
  return result
}

export async function runWithCleanup(action, cleanup) {
  let value
  let failure
  try { value = await action() } catch (error) { failure = error }
  try { await cleanup() } catch (error) {
    failure = failure
      ? new AggregateError([failure, error], `${failure.message}; ${error.message}`)
      : error
  }
  if (failure) throw failure
  return value
}

export async function waitForHttpReady(url, { timeoutMs = 5_000, fetchImpl = fetch } = {}) {
  const deadline = Date.now() + timeoutMs
  let lastError = "no response"
  while (Date.now() < deadline) {
    const remaining = Math.max(1, deadline - Date.now())
    try {
      const response = await fetchImpl(url, { signal: AbortSignal.timeout(remaining) })
      if (response.ok) return
      lastError = `HTTP ${response.status}`
      await response.body?.cancel()
    } catch (error) { lastError = error.message }
    const wait = Math.min(100, deadline - Date.now())
    if (wait > 0) await new Promise(resolve => setTimeout(resolve, wait))
  }
  throw new Error(`Production preview HTTP readiness timed out after ${timeoutMs}ms: ${lastError}`)
}

function createTemporaryBrowserProfile() {
  return mkdtempSync(join(tmpdir(), "ai-token-meter-density-"))
}

function removeTemporaryBrowserProfile(profileDirectory) {
  rmSync(profileDirectory, {
    recursive: true,
    force: true,
    maxRetries: 3,
    retryDelay: 50,
  })
}

function signalProcessGroup(pid, signal, killProcess) {
  try {
    killProcess(-pid, signal)
  } catch (error) {
    if (error?.code !== "ESRCH") throw error
  }
}

function isProcessGroupAlive(pid, killProcess) {
  try {
    killProcess(-pid, 0)
    return true
  } catch (error) {
    if (error?.code === "ESRCH") return false
    if (error?.code === "EPERM") return true
    throw error
  }
}

async function waitForProcessGroupExit(pid, timeoutMs, {
  killProcess = process.kill,
} = {}) {
  const deadline = Date.now() + timeoutMs
  while (Date.now() < deadline) {
    if (!isProcessGroupAlive(pid, killProcess)) return true
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 25))
  }
  return !isProcessGroupAlive(pid, killProcess)
}

function hasExited(child) {
  return child.exitCode !== null || child.signalCode !== null
}

export function waitForExit(child, timeoutMs) {
  if (hasExited(child)) return Promise.resolve(true)
  return new Promise((resolveExit) => {
    const timeout = setTimeout(() => resolveExit(false), timeoutMs)
    child.once("exit", () => {
      clearTimeout(timeout)
      resolveExit(true)
    })
  })
}

export function runBoundedCommand(command, args, {timeoutMs = 3_000, spawnImpl = spawn} = {}) {
  return new Promise((resolveProcess, reject) => {
    const child = spawnImpl(command, args, { stdio: "ignore" })
    const timeout = setTimeout(() => {
      try { child.kill("SIGKILL") } catch { /* Still report the bounded helper failure. */ }
      const error = new Error(`${command} timed out after ${timeoutMs}ms; helper termination requested`)
      error.code = "PROCESS_TIMEOUT"
      reject(error)
    }, timeoutMs)
    child.once("error", error => { clearTimeout(timeout); reject(error) })
    child.once("exit", (code) => {
      clearTimeout(timeout)
      if (code === 0 || code === 128) resolveProcess()
      else reject(new Error(`${command} exited ${code}`))
    })
  })
}

import { spawn } from "node:child_process"

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

export async function stopProcessTree(child, {
  platform = process.platform,
  killProcess = process.kill,
  runProcess = runCommand,
  waitForExit: wait = waitForExit,
  timeoutMs = 3_000,
} = {}) {
  if (!child.pid) return

  if (platform === "win32") {
    try {
      await runProcess("taskkill", ["/pid", String(child.pid), "/T", "/F"])
    } catch (error) {
      if (!hasExited(child)) throw error
    }
    if (!await wait(child, timeoutMs)) throw new Error("Process tree did not exit after cleanup")
    return
  }

  signalProcessGroup(child.pid, "SIGTERM", killProcess)
  if (await wait(child, timeoutMs)) return
  signalProcessGroup(child.pid, "SIGKILL", killProcess)
  if (!await wait(child, timeoutMs)) throw new Error("Process tree did not exit after cleanup")
}

export async function runBrowser(executable, url, {
  timeoutMs = 15_000,
  platform = process.platform,
  spawnImpl = spawn,
  stopProcessTreeImpl = stopProcessTree,
} = {}) {
  const browser = spawnManagedProcess(executable, ["--headless=new", "--disable-gpu", "--dump-dom", "--virtual-time-budget=1000", url], {
    stdio: ["ignore", "pipe", "pipe"],
  }, { platform, spawnImpl })
  let timeout

  try {
    return await new Promise((resolveOutput, reject) => {
      let output = ""
      let errors = ""
      browser.stdout.on("data", (chunk) => { output += chunk })
      browser.stderr.on("data", (chunk) => { errors += chunk })
      browser.once("error", reject)
      browser.once("exit", (code) => code === 0
        ? resolveOutput(output)
        : reject(new Error(`Browser exited ${code}: ${errors}`)))
      timeout = setTimeout(() => reject(new Error(`Browser timed out after ${timeoutMs}ms`)), timeoutMs)
    })
  } finally {
    clearTimeout(timeout)
    await stopProcessTreeImpl(browser, { platform })
  }
}

function signalProcessGroup(pid, signal, killProcess) {
  try {
    killProcess(-pid, signal)
  } catch (error) {
    if (error?.code !== "ESRCH") throw error
  }
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

function runCommand(command, args) {
  return new Promise((resolveProcess, reject) => {
    const child = spawn(command, args, { stdio: "ignore" })
    child.once("error", reject)
    child.once("exit", (code) => code === 0 || code === 128
      ? resolveProcess()
      : reject(new Error(`${command} exited ${code}`)))
  })
}

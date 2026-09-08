import assert from "node:assert/strict"
import { EventEmitter } from "node:events"
import { join } from "node:path"
import test from "node:test"

import {
  extractPreviewUrl,
  runBrowser,
  runBoundedCommand,
  runWithCleanup,
  waitForHttpReady,
  spawnDensityPreview,
  spawnManagedProcess,
  stopProcessTree,
} from "./density-process-lifecycle.mjs"

class FakeChild extends EventEmitter {
  constructor(pid = 4321) {
    super()
    this.pid = pid
    this.exitCode = null
    this.signalCode = null
    this.stdout = new EventEmitter()
    this.stderr = new EventEmitter()
  }
}

test("starts Unix commands in their own process group", () => {
  const child = new FakeChild()
  let options

  const result = spawnManagedProcess("vite", ["--port", "0"], {
    cwd: "/fixture",
    stdio: "ignore",
  }, {
    platform: "darwin",
    spawnImpl: (_command, _args, receivedOptions) => {
      options = receivedOptions
      return child
    },
  })

  assert.equal(result, child)
  assert.equal(options.detached, true)
})

test("timeout diagnostics distinguish browser exit from open streams and retain bounded stderr", async () => {
  const child = new FakeChild()
  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: 1,
    spawnImpl: () => child,
    stopProcessTreeImpl: async () => {},
  })
  child.stderr.emit("data", "x".repeat(9000) + "browser diagnostic tail")
  child.stdout.emit("data", "partial DOM")
  child.exitCode = 0
  child.emit("exit", 0)
  await assert.rejects(pending, error => {
    assert.match(error.message, /pid=4321/)
    assert.match(error.message, /stdoutBytes=11/)
    assert.match(error.message, /exit=0/)
    assert.match(error.message, /close=not observed/)
    assert.match(error.message, /browser diagnostic tail/)
    assert.ok(error.message.length < 5000)
    return true
  })
})

test("browser timeout remains visible if process cleanup also fails", async () => {
  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: 1,
    spawnImpl: () => new FakeChild(),
    stopProcessTreeImpl: async () => { throw new Error("cleanup fixture failure") },
  })
  await assert.rejects(pending, error => {
    assert.match(error.message, /Browser timed out/)
    assert.match(error.message, /cleanup fixture failure/)
    return true
  })
})

test("Windows cleanup command is bounded and terminates a stuck taskkill helper", async () => {
  const child = new FakeChild()
  const signals = []
  child.kill = signal => { signals.push(signal); return true }
  // End the fixture even with the old unbounded implementation, so red is finite.
  const fallback = setTimeout(() => child.emit("exit", 0), 50)
  try {
    await assert.rejects(runBoundedCommand("taskkill", ["/pid", "123", "/T", "/F"], {
      timeoutMs: 1,
      spawnImpl: () => child,
    }), /taskkill timed out after 1ms/)
  } finally { clearTimeout(fallback) }
  assert.deepEqual(signals, ["SIGKILL"])
})

test("a fresh Windows profile can finish after 15 seconds but still has a finite deadline", async () => {
  const child = new FakeChild()
  let timer
  let now = 0
  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    platform: "win32",
    now: () => now,
    setTimer: (callback, delay) => { timer = {callback, delay}; return 1 },
    clearTimer: () => {},
    spawnImpl: () => child,
    createBrowserProfile: () => "/fixture/profile",
    removeBrowserProfile: () => {},
    stopProcessTreeImpl: async () => {},
  })
  now = 30_000
  if (now >= timer.delay) timer.callback()
  child.stdout.emit("data", "<html>density report</html>")
  child.exitCode = 0
  child.emit("close", 0)
  assert.equal(await pending, "<html>density report</html>")
  assert.ok(timer.delay <= 45_000)
})

test("a preview HTTP request that never responds is aborted within its startup budget", async () => {
  let aborted = false
  // Keep the event loop live while AbortSignal's unref'ed deadline runs.
  const keepAlive = setTimeout(() => {}, 100)
  try {
    await assert.rejects(waitForHttpReady("http://127.0.0.1:4173", {
      timeoutMs: 1,
      fetchImpl: (_url, { signal }) => new Promise((_resolve, reject) => {
        signal.addEventListener("abort", () => { aborted = true; reject(signal.reason) })
      }),
    }), /preview HTTP readiness timed out/)
    assert.equal(aborted, true)
  } finally { clearTimeout(keepAlive) }
})

test("a failed typography assertion cannot be masked by preview cleanup failure", async () => {
  await assert.rejects(runWithCleanup(
    async () => { throw new Error("detailBody expected 13px, received 14px") },
    async () => { throw new Error("preview cleanup failed") },
  ), error => {
    assert.match(error.message, /detailBody expected 13px, received 14px/)
    assert.match(error.message, /preview cleanup failed/)
    return true
  })
})

test("a taskkill timeout is not mistaken for success when the parent browser already exited", async () => {
  const child = new FakeChild()
  child.exitCode = 0
  await assert.rejects(stopProcessTree(child, {
    platform: "win32",
    runProcess: async () => { const error = new Error("taskkill timed out"); error.code = "PROCESS_TIMEOUT"; throw error },
    waitForExit: async () => true,
  }), /taskkill timed out/)
})

test("an oversized DOM fails instead of growing the browser output buffer without limit", async () => {
  const child = new FakeChild()
  let cleaned = false
  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    spawnImpl: () => child,
    stopProcessTreeImpl: async () => { cleaned = true },
  })
  child.stdout.emit("data", Buffer.alloc(2 * 1024 * 1024 + 1, "x"))
  child.exitCode = 0
  child.emit("close", 0)
  await assert.rejects(pending, /stdout exceeded 2 MiB/)
  assert.equal(cleaned, true)
})

test("the default Windows deadline still fails a stalled browser and removes its isolated profile", async () => {
  const child = new FakeChild()
  let deadline
  const cleanup = []
  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    platform: "win32",
    setTimer: callback => { deadline = callback; return 1 },
    clearTimer: () => {},
    spawnImpl: () => child,
    createBrowserProfile: () => "/fixture/profile",
    removeBrowserProfile: () => { cleanup.push("profile") },
    stopProcessTreeImpl: async () => { cleanup.push("process tree") },
  })
  deadline()
  await assert.rejects(pending, /Browser timed out after 45000ms/)
  assert.deepEqual(cleanup, ["process tree", "profile"])
})

test("starts the density server from the production preview command", () => {
  const child = new FakeChild()
  let invocation

  const result = spawnDensityPreview("/fixture/windows", {
    platform: "darwin",
    spawnImpl: (command, args, options) => {
      invocation = { command, args, options }
      return child
    },
  })

  assert.equal(result, child)
  assert.equal(invocation.command, process.execPath)
  assert.deepEqual(invocation.args, [
    join("/fixture/windows", "node_modules", "vite", "bin", "vite.js"),
    "preview", "--config", "vite.density.config.ts", "--host", "127.0.0.1", "--port", "0",
  ])
  assert.equal(invocation.options.cwd, "/fixture/windows")
})

test("starts Windows preview through Node instead of an unspawnable cmd shim", () => {
  const child = new FakeChild()
  let invocation
  const windowsRoot = join("fixture", "windows")
  const nodeExecutable = join("fixture", "nodejs", "node.exe")

  spawnDensityPreview(windowsRoot, {
    platform: "win32",
    nodeExecutable,
    spawnImpl: (command, args, options) => {
      invocation = { command, args, options }
      return child
    },
  })

  assert.equal(invocation.command, nodeExecutable)
  assert.equal(invocation.args[0], join(windowsRoot, "node_modules", "vite", "bin", "vite.js"))
  assert.equal(invocation.options.detached, false)
})

test("extracts a random preview port from ANSI-decorated Windows output", () => {
  const output = "  \u001b[32m➜\u001b[39m  \u001b[1mLocal\u001b[22m:   \u001b[36mhttp://127.0.0.1:\u001b[1m62072\u001b[22m/\u001b[39m"

  assert.equal(extractPreviewUrl(output), "http://127.0.0.1:62072/")
})

test("terminates a Unix process group with TERM then KILL when it will not exit", async () => {
  const child = new FakeChild()
  const signals = []

  await assert.rejects(
    stopProcessTree(child, {
      platform: "darwin",
      killProcess: (pid, signal) => signals.push([pid, signal]),
      waitForExit: async () => false,
      waitForGroupExit: async () => false,
    }),
    /did not exit after cleanup/,
  )

  assert.deepEqual(signals, [[-4321, "SIGTERM"], [-4321, "SIGKILL"]])
})

test("kills surviving Unix descendants even when the process-group leader exits", async () => {
  const child = new FakeChild()
  const signals = []

  await stopProcessTree(child, {
    platform: "darwin",
    killProcess: (pid, signal) => {
      signals.push([pid, signal])
      if (signal === 0) return
    },
    waitForExit: async () => {
      child.exitCode = 0
      return true
    },
    waitForGroupExit: async () => signals.some(([, signal]) => signal === "SIGKILL"),
  })

  assert.deepEqual(signals, [
    [-4321, "SIGTERM"],
    [-4321, "SIGKILL"],
  ])
})

test("allows Unix descendants the full TERM grace period before deciding to KILL", async () => {
  const child = new FakeChild()
  const signals = []

  await stopProcessTree(child, {
    platform: "darwin",
    killProcess: (pid, signal) => { signals.push([pid, signal]) },
    waitForExit: async () => {
      child.exitCode = 0
      return true
    },
    waitForGroupExit: async () => true,
  })

  assert.deepEqual(signals, [[-4321, "SIGTERM"]])
})

test("times out a stalled browser and always invokes process-tree cleanup", async () => {
  const child = new FakeChild()
  let cleanupCount = 0

  await assert.rejects(
    runBrowser("browser", "http://127.0.0.1:4173", {
      timeoutMs: 1,
      spawnImpl: () => child,
      stopProcessTreeImpl: async (receivedChild) => {
        cleanupCount += 1
        assert.equal(receivedChild, child)
      },
    }),
    /Browser timed out after 1ms/,
  )

  assert.equal(cleanupCount, 1)
})

test("cleans up a browser after its normal exit", async () => {
  const child = new FakeChild()
  let cleanupCount = 0

  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: 100,
    spawnImpl: () => child,
    stopProcessTreeImpl: async () => { cleanupCount += 1 },
  })
  child.exitCode = 0
  child.emit("exit", 0)
  child.emit("close", 0)

  assert.equal(await pending, "")
  assert.equal(cleanupCount, 1)
})

test("waits for browser streams to close before returning dumped DOM", async () => {
  const child = new FakeChild()
  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: 100,
    spawnImpl: () => child,
    stopProcessTreeImpl: async () => {},
  })

  child.exitCode = 0
  child.emit("exit", 0)
  child.stdout.emit("data", Buffer.from("<html>complete</html>"))
  child.emit("close", 0)

  assert.equal(await pending, "<html>complete</html>")
})

test("runs the browser with an isolated disposable profile", async () => {
  const child = new FakeChild()
  let invocation
  const removedProfiles = []

  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: 100,
    platform: "win32",
    spawnImpl: (command, args, options) => {
      invocation = { command, args, options }
      return child
    },
    createBrowserProfile: () => "/fixture/isolated-profile",
    removeBrowserProfile: (path) => { removedProfiles.push(path) },
    stopProcessTreeImpl: async () => {},
  })

  child.exitCode = 0
  child.emit("close", 0)
  await pending

  assert.equal(invocation.command, "browser")
  assert.ok(invocation.args.includes("--user-data-dir=/fixture/isolated-profile"))
  assert.ok(invocation.args.includes("--no-first-run"))
  assert.ok(invocation.args.includes("--no-default-browser-check"))
  assert.ok(invocation.args.includes("--disable-extensions"))
  assert.deepEqual(removedProfiles, ["/fixture/isolated-profile"])
})

test("bounds DOM capture inside the external Windows browser watchdog", async () => {
  const child = new FakeChild()
  let invocation
  const watchdogMs = 45_000

  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: watchdogMs,
    platform: "win32",
    spawnImpl: (command, args, options) => {
      invocation = { command, args, options }
      return child
    },
    createBrowserProfile: () => "/fixture/isolated-profile",
    removeBrowserProfile: () => {},
    stopProcessTreeImpl: async () => {},
  })

  child.exitCode = 0
  child.emit("close", 0)
  await pending

  const deadlineArgument = invocation.args.find(argument => argument.startsWith("--timeout="))
  assert.ok(deadlineArgument, "headless DOM capture must have its own deadline")
  const captureMs = Number(deadlineArgument.slice("--timeout=".length))
  assert.ok(Number.isInteger(captureMs) && captureMs > 0)
  assert.ok(captureMs <= 15_000)
  assert.ok(captureMs < watchdogMs)
})

test("removes the disposable profile even when process-tree cleanup fails", async () => {
  const child = new FakeChild()
  const removedProfiles = []

  const pending = runBrowser("browser", "http://127.0.0.1:4173", {
    timeoutMs: 100,
    platform: "win32",
    spawnImpl: () => child,
    createBrowserProfile: () => "/fixture/isolated-profile",
    removeBrowserProfile: (path) => { removedProfiles.push(path) },
    stopProcessTreeImpl: async () => { throw new Error("cleanup failed") },
  })

  child.exitCode = 0
  child.emit("close", 0)

  await assert.rejects(pending, /cleanup failed/)
  assert.deepEqual(removedProfiles, ["/fixture/isolated-profile"])
})

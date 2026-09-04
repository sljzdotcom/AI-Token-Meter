import assert from "node:assert/strict"
import { EventEmitter } from "node:events"
import { join } from "node:path"
import test from "node:test"

import {
  runBrowser,
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
  child.emit("exit", 0)

  assert.equal(await pending, "")
  assert.equal(cleanupCount, 1)
})

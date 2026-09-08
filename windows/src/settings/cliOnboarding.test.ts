import { describe, expect, it } from "vitest"
import { CLIOnboarding, serviceAction } from "./cliOnboarding"
import type { ServiceAccountStatus } from "./SettingsWindow"

describe("CLI onboarding", () => {
  it("only an explicit status check requests a quota retry", async () => {
    const calls: Array<{command: string; retryUsage?: boolean}> = []
    const controller = new CLIOnboarding(async (command, args) => {
      calls.push({command, retryUsage: args.retryUsage})
      return {providerId: args.providerId, connectionState: "connected"}
    }, () => {}, () => {}, () => {})
    await controller.check("claude")
    await controller.check("claude", true)
    expect(calls).toEqual([
      {command: "service_account_status", retryUsage: false},
      {command: "service_account_status", retryUsage: true},
    ])
  })
  it("initialization is available for a connected Claude account and waits for manual checking", async () => {
    const commands: string[] = []
    const messages: string[] = []
    const controller = new CLIOnboarding(async (command, args) => {
      commands.push(command)
      if (command === "service_account_status") return {providerId: args.providerId, connectionState: "connected", accountLabel: "kept"}
      if (command === "begin_claude_usage_initialization") return null
      throw Error("unexpected command")
    }, () => {}, value => messages.push(value), () => {})
    await controller.initializeClaudeUsage()
    expect(commands).toEqual(["service_account_status", "begin_claude_usage_initialization"])
    expect(messages).toEqual(["Complete Claude Code workspace setup in Terminal, then choose Check Status."])
    expect(controller.isBusy("claude")).toBe(false)
  })
  it("initialization failure clears busy state and gives retry instructions", async () => {
    const messages: string[] = []
    const controller = new CLIOnboarding(async command => {
      if (command === "service_account_status") return {providerId: "claude", connectionState: "connected"}
      throw Error("fixture")
    }, () => {}, value => messages.push(value), () => {})
    await controller.initializeClaudeUsage()
    expect(messages.at(-1)).toBe("The Claude Code setup window could not be opened. Choose Check Status or retry initialization.")
    expect(controller.isBusy("claude")).toBe(false)
  })
  it("missing invites installation, unavailable retries, connected stays neutral", () => {
    expect(serviceAction("notInstalled", false)).toEqual({ title: "Install CLI", attention: true, disabled: false })
    expect(serviceAction("unavailable", false)).toEqual({ title: "Check Status", attention: false, disabled: false })
    expect(serviceAction("connected", false)).toEqual({ title: "Sign in again", attention: false, disabled: false })
    expect(serviceAction("signInRequired", false).attention).toBe(true)
    expect(serviceAction("checking", false).disabled).toBe(true)
    expect(serviceAction("notInstalled", true).disabled).toBe(true)
  })
  it("rechecks installation, rejects repeats and stops after discovery without login", async () => {
    let release!: () => void
    const wait = new Promise<void>(resolve => { release = resolve })
    let reads = 0
    const commands: string[] = []
    const statuses: ServiceAccountStatus[] = []
    const controller = new CLIOnboarding(async (command) => {
      commands.push(command)
      if (command === "begin_service_installation") return "launched"
      if (command !== "service_account_status") throw Error("unexpected login")
      return { providerId: "claude", connectionState: ++reads === 1 ? "notInstalled" : "signInRequired" }
    }, status => statuses.push(status), () => {}, () => {}, () => wait, 3)
    const operation = controller.begin("claude", "install")
    await controller.begin("claude", "install")
    await controller.begin("claude", "login")
    release()
    await operation
    expect(commands).toEqual(["service_account_status", "begin_service_installation", "service_account_status"])
    expect(statuses.at(-1)?.connectionState).toBe("signInRequired")
    expect(controller.isBusy("claude")).toBe(false)
  })
  it("preserves existing runtime and account", async () => {
    const commands: string[] = []
    const controller = new CLIOnboarding(async command => { commands.push(command); return {providerId: "codex", connectionState: "connected", accountLabel: "kept"} }, () => {}, () => {}, () => {}, async () => {}, 2)
    await controller.begin("codex", "install")
    expect(commands).toEqual(["service_account_status"])
  })
  it("launch failure and timeout both allow retry", async () => {
    let launches = 0
    const controller = new CLIOnboarding(async command => {
      if (command === "begin_service_installation") { if (++launches === 1) throw Error("fixture"); return "launched" }
      return {providerId: "claude", connectionState: "notInstalled"}
    }, () => {}, () => {}, () => {}, async () => {}, 2)
    await controller.begin("claude", "install")
    expect(controller.isBusy("claude")).toBe(false)
    await controller.begin("claude", "install")
    expect(launches).toBe(2)
    expect(controller.isBusy("claude")).toBe(false)
  })
  it("closing ignores pending reads and stops further polling", async () => {
    let release!: (value: unknown) => void
    const pending = new Promise(resolve => { release = resolve })
    const statuses: ServiceAccountStatus[] = []
    const commands: string[] = []
    const controller = new CLIOnboarding(async command => { commands.push(command); return pending }, status => statuses.push(status), () => {}, () => {}, async () => {}, 2)
    const operation = controller.begin("codex", "install")
    controller.dispose()
    release({providerId: "codex", connectionState: "notInstalled"})
    await operation
    expect(commands).toEqual(["service_account_status"])
    expect(statuses).toEqual([])
    expect(controller.isBusy("codex")).toBe(false)
  })
  it("unavailable recheck cannot be mistaken for a missing CLI", async () => {
    const commands: string[] = []
    const controller = new CLIOnboarding(async command => { commands.push(command); throw Error("unavailable") }, () => {}, () => {}, () => {}, async () => {}, 2)
    await controller.begin("codex", "install")
    expect(commands).toEqual(["service_account_status"])
  })
  it("backend discovery becoming unavailable stops installation polling and offers recovery", async () => {
    const statuses: ServiceAccountStatus[] = []
    let reads = 0
    const controller = new CLIOnboarding(async command => {
      if (command === "begin_service_installation") return "unavailable"
      reads++
      return {providerId: "codex", connectionState: "notInstalled"}
    }, status => statuses.push(status), () => {}, () => {}, async () => {}, 2)
    await controller.begin("codex", "install")
    expect(statuses.at(-1)?.connectionState).toBe("unavailable")
    expect(reads).toBe(1)
    expect(controller.isBusy("codex")).toBe(false)
  })
})

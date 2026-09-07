import { describe, expect, it } from "vitest"
import { CLIOnboarding, serviceAction } from "./cliOnboarding"
import type { ServiceAccountStatus } from "./SettingsWindow"

describe("CLI onboarding", () => {
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
})

import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import type { Code0Action, Code0ActionResult, Code0SessionBridge } from "./code0Session"

export type Code0SessionTransport = {
  invoke(command: string, args?: Record<string, unknown>): Promise<unknown>
  listen(event: string, handler: (value: unknown) => void): Promise<() => void>
}

const tauriTransport: Code0SessionTransport = {
  invoke: (command, args) => args === undefined ? invoke<unknown>(command) : invoke<unknown>(command, args),
  listen: (event, handler) => listen<unknown>(event, message => handler(message.payload)),
}

export class TauriCode0SessionBridge implements Code0SessionBridge {
  constructor(private readonly transport: Code0SessionTransport = tauriTransport) {}
  read(): Promise<unknown> { return this.transport.invoke("code0_session_state") }
  onUpdated(handler: (value: unknown) => void): Promise<() => void> {
    return this.transport.listen("code0-session-updated", handler)
  }
  async perform(action: Code0Action): Promise<Code0ActionResult> {
    if (action !== "connect" && action !== "loginCompleted" && action !== "syncNow" && action !== "disconnect") return {kind: "denied"}
    try {
      return decodeActionResult(await this.transport.invoke("code0_session_action", {action}))
    } catch { return {kind: "denied"} }
  }
}

function decodeActionResult(value: unknown): Code0ActionResult {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return {kind: "denied"}
  const prototype = Object.getPrototypeOf(value)
  if (prototype !== Object.prototype && prototype !== null) return {kind: "denied"}
  const kind = Object.getOwnPropertyDescriptor(value, "kind")
  if (!kind || !("value" in kind)) return {kind: "denied"}
  const keys = Reflect.ownKeys(value)
  if (kind.value === "started" || kind.value === "busy" || kind.value === "denied") {
    return keys.length === 1 ? Object.freeze({kind: kind.value}) : {kind: "denied"}
  }
  if (kind.value !== "throttled" || keys.length !== 2 || !keys.includes("secondsRemaining")) return {kind: "denied"}
  const seconds = Object.getOwnPropertyDescriptor(value, "secondsRemaining")
  if (!seconds || !("value" in seconds) || typeof seconds.value !== "number"
    || !Number.isSafeInteger(seconds.value) || seconds.value < 0 || seconds.value > 300) return {kind: "denied"}
  return Object.freeze({kind: "throttled", secondsRemaining: seconds.value})
}

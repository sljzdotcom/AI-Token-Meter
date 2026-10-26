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

// Read-only RED seam. No surface installs this adapter yet; actions remain denied
// until the integration's compiled behavioral RED has been reviewed.
export class TauriCode0SessionBridge implements Code0SessionBridge {
  constructor(private readonly transport: Code0SessionTransport = tauriTransport) {}
  read(): Promise<unknown> { return this.transport.invoke("code0_session_state") }
  onUpdated(handler: (value: unknown) => void): Promise<() => void> {
    return this.transport.listen("code0-session-updated", handler)
  }
  async perform(_action: Code0Action): Promise<Code0ActionResult> { return {kind: "denied"} }
}

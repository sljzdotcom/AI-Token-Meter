import { expect, it, vi } from "vitest"
import { TauriCode0SessionBridge, type Code0SessionTransport } from "./tauriCode0SessionBridge"
import type { Code0Action, Code0ActionResult } from "./code0Session"
import { code0Session } from "../test/code0SessionFixtures"

function transport(reply: unknown) {
  const stop = vi.fn()
  let emit: (value: unknown) => void = () => {}
  const invoke = vi.fn(async (_command: string, _args?: Record<string, unknown>): Promise<unknown> => reply)
  const listen = vi.fn(async (_event: string, handler: (value: unknown) => void) => {
    emit = handler
    return stop
  })
  const bridge = new TauriCode0SessionBridge({invoke, listen} satisfies Code0SessionTransport)
  return {bridge, invoke, listen, stop, emit: (value: unknown) => emit(value)}
}

it.each<Code0Action>(["connect", "loginCompleted", "syncNow", "disconnect"])("maps explicit %s to only the fixed action command and exact parameter", async action => {
  const fixture = transport({kind: "started"})
  expect(await fixture.bridge.perform(action)).toEqual({kind: "started"})
  expect(fixture.invoke.mock.calls).toEqual([["code0_session_action", {action}]])
  expect(fixture.listen).not.toHaveBeenCalled()
})

it("reads the state command without any parameter or action", async () => {
  const value = code0Session()
  const fixture = transport(value)
  expect(await fixture.bridge.read()).toBe(value)
  expect(fixture.invoke.mock.calls).toEqual([["code0_session_state"]])
})

it("subscribes only to the state event and releases the real listener", async () => {
  const fixture = transport({kind: "started"})
  const handler = vi.fn()
  const stop = await fixture.bridge.onUpdated(handler)
  const value = code0Session({revision: 2, snapshot: null})
  fixture.emit(value)
  expect(handler.mock.calls).toEqual([[value]])
  expect(fixture.listen.mock.calls[0][0]).toBe("code0-session-updated")
  expect(fixture.invoke).not.toHaveBeenCalled()
  stop()
  expect(fixture.stop).toHaveBeenCalledOnce()
})

it.each<Code0ActionResult>([{kind: "busy"}, {kind: "denied"}, {kind: "throttled", secondsRemaining: 300}])("returns the fixed native reply $kind without performing a read or fallback", async reply => {
  const fixture = transport(reply)
  expect(await fixture.bridge.perform("syncNow")).toEqual(reply)
  expect(fixture.invoke.mock.calls).toEqual([["code0_session_action", {action: "syncNow"}]])
})

it.each([null, undefined, {}, {kind: "started", account: "discard"}, {kind: "throttled", secondsRemaining: -1}, {kind: "throttled", secondsRemaining: 301}, {kind: "throttled", secondsRemaining: 1.5}, {kind: "throttled", secondsRemaining: true}, {kind: "throttled", secondsRemaining: "1"}])("invalid native reply denies without fallback (%j)", async reply => {
  const fixture = transport(reply)
  expect(await fixture.bridge.perform("connect")).toEqual({kind: "denied"})
  expect(fixture.invoke.mock.calls).toEqual([["code0_session_action", {action: "connect"}]])
})

it("a missing backend denies without trying any other command", async () => {
  const fixture = transport({kind: "started"})
  fixture.invoke.mockRejectedValueOnce(new Error("controlled-unavailable"))
  expect(await fixture.bridge.perform("connect")).toEqual({kind: "denied"})
  expect(fixture.invoke.mock.calls).toEqual([["code0_session_action", {action: "connect"}]])
})

it("an unknown action is rejected before any invoke", async () => {
  const fixture = transport({kind: "started"})
  expect(await fixture.bridge.perform("refresh" as Code0Action)).toEqual({kind: "denied"})
  expect(fixture.invoke).not.toHaveBeenCalled()
})

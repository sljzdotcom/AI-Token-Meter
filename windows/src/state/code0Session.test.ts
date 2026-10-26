import { expect, it } from "vitest"
import { code0SessionFailures, code0SessionStates, decodeCode0Session, disabledCode0SessionBridge, emptyCode0Session } from "./code0Session"
import { code0Session, code0Snapshot } from "../test/code0SessionFixtures"

it.each(code0SessionStates)("decodes the independent %s state including explicit null snapshot", state => {
  const value = code0Session({state, snapshot: null})
  expect(decodeCode0Session(value)).toEqual(value)
})

it.each(code0SessionFailures)("decodes fixed failure %s without a raw error string", failure => {
  const value = code0Session({failure, snapshot: null})
  expect(decodeCode0Session(value)).toEqual(value)
})

it("accepts committed zero site credits and native successful time without a ratio", () => {
  const value = code0Session({snapshot: code0Snapshot(0, 0), retryAfterSeconds: 300})
  expect(decodeCode0Session(value)).toEqual(value)
})

it.each([null, {}, {providerId: "code0", connectionState: "connected"},
  {...code0Session(), revision: -1}, {...code0Session(), revision: 1.5},
  {...code0Session(), revision: Number.MAX_SAFE_INTEGER + 1}, {...code0Session(), revision: NaN},
  {...code0Session(), state: "loggedIn"}, {...code0Session(), failure: "arbitrary remote text"},
  {...code0Session(), isAutomaticallySyncing: "true"}, {...code0Session(), retryAfterSeconds: -1},
  {...code0Session(), retryAfterSeconds: Infinity}, {...code0Session(), snapshot: undefined},
  {...code0Session(), epoch: "must-not-leave-native"}, {...code0Session(), nonce: "must-not-leave-native"},
  {...code0Session(), account: "must-not-enter-ui"}, {...code0Session(), body: {}},
])("rejects malformed or expanded session projections %#", value => {
  expect(decodeCode0Session(value)).toBeNull()
})

it.each([
  {...code0Snapshot(), providerId: "deepseek"},
  {...code0Snapshot(), usedRatio: 0.5},
  {...code0Snapshot(), fetchedAt: "not-a-native-time"},
  {...code0Snapshot(), primaryMetric: {...code0Snapshot().primaryMetric, current: -1}},
  {...code0Snapshot(), primaryMetric: {...code0Snapshot().primaryMetric, current: Infinity}},
  {...code0Snapshot(), primaryMetric: {...code0Snapshot().primaryMetric, current: "2.5"}},
  {...code0Snapshot(), primaryMetric: {...code0Snapshot().primaryMetric, current: true}},
  {...code0Snapshot(), primaryMetric: {...code0Snapshot().primaryMetric, unit: "usd"}},
  {...code0Snapshot(), secondaryMetric: {...code0Snapshot().secondaryMetric, kind: "localBudget"}},
  {...code0Snapshot(), secondaryMetric: null},
  {...code0Snapshot(), accountId: "not-a-numeric-field"},
])("rejects non-Code0 numeric snapshots without currency or silent coercion %#", snapshot => {
  expect(decodeCode0Session({...code0Session(), snapshot})).toBeNull()
})

it("default bridge is inert and denies all explicit actions", async () => {
  expect(await disabledCode0SessionBridge.read()).toEqual(emptyCode0Session)
  for (const action of ["connect", "loginCompleted", "syncNow", "disconnect"] as const) {
    expect(await disabledCode0SessionBridge.perform(action)).toEqual({kind: "denied"})
  }
  const stop = await disabledCode0SessionBridge.onUpdated(() => { throw new Error("No implicit event") })
  stop()
})

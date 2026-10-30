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


it("clones and freezes every accepted numeric projection so source mutation cannot rewrite it", () => {
  const source = {...code0Session(), snapshot: {...code0Snapshot(), primaryMetric: {...code0Snapshot().primaryMetric}}}
  const accepted = decodeCode0Session(source)
  expect(accepted).not.toBeNull()
  expect(accepted).not.toBe(source)
  expect(accepted?.snapshot).not.toBe(source.snapshot)
  expect(Object.isFrozen(accepted)).toBe(true)
  expect(Object.isFrozen(accepted?.snapshot)).toBe(true)
  expect(Object.isFrozen(accepted?.snapshot?.primaryMetric)).toBe(true)
  expect(Object.isFrozen(accepted?.snapshot?.secondaryMetric)).toBe(true)
  source.revision = 99
  source.snapshot.primaryMetric.current = 99
  expect(accepted?.revision).toBe(1)
  expect(accepted?.snapshot?.primaryMetric.current).toBe(2.5)
})

it.each([0, Number.MAX_SAFE_INTEGER])("accepts safe revision boundary %s without coercion", revision => {
  const value = code0Session({revision})
  expect(decodeCode0Session(value)).toEqual(value)
})

it("rejects accessors and extra numeric fields without evaluating arbitrary getters", () => {
  let reads = 0
  const source = {...code0Session()}
  Object.defineProperty(source, "state", {enumerable: true, get() { reads += 1; return "paused" }})
  expect(decodeCode0Session(source)).toBeNull()
  expect(reads).toBe(0)
  const snapshot = code0Snapshot()
  expect(decodeCode0Session({...code0Session(), snapshot: {...snapshot,
    primaryMetric: {...snapshot.primaryMetric, account: "discard"}}})).toBeNull()
})

it.each([0.5, 301, NaN])("rejects invalid presentation retry value %s", retryAfterSeconds => {
  expect(decodeCode0Session({...code0Session(), retryAfterSeconds})).toBeNull()
})

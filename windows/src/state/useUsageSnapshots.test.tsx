import { act, renderHook, waitFor } from "@testing-library/react"
import { expect, it, vi } from "vitest"
import { useUsageSnapshots } from "./useUsageSnapshots"
import type { UsageSnapshot } from "./usage"
import { unavailableSnapshots } from "./usage"
import type { UsageBridge } from "./usageBridge"
import type { Code0SessionDTO } from "./code0Session"
import { emptyCode0Session } from "./code0Session"
import { code0Session, code0Snapshot, deferred } from "../test/code0SessionFixtures"

const ordinary = unavailableSnapshots.filter(value => value.providerId !== "code0")
const oldCode0: UsageSnapshot = code0Snapshot(101, 202)

function fixture(read: Promise<UsageSnapshot[]>) {
  const handlers = new Set<(value: UsageSnapshot) => void>()
  const stop = vi.fn()
  const bridge: UsageBridge = {
    readSnapshots: vi.fn(() => read),
    onSnapshotUpdated: vi.fn(async handler => {
      handlers.add(handler)
      return () => { handlers.delete(handler); stop() }
    }),
  }
  return {bridge, stop, emit(value: UsageSnapshot) { handlers.forEach(handler => handler(value)) }}
}

function code0(values: UsageSnapshot[]) { return values.find(value => value.providerId === "code0") }

it("current session B wins a late ordinary read containing A", async () => {
  const read = deferred<UsageSnapshot[]>()
  const input = fixture(read.promise)
  const current = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
  const {result} = renderHook(() => useUsageSnapshots(input.bridge, undefined, current))
  await waitFor(() => expect(input.bridge.onSnapshotUpdated).toHaveBeenCalledOnce())
  await act(async () => { read.resolve([...ordinary, oldCode0]); await read.promise })
  expect(code0(result.current)).toEqual(current.snapshot)
  expect(result.current.map(value => value.providerId)).toEqual(["claude", "codex", "deepseek", "gemini", "code0"])
})

it("explicit current null cannot be revived by a late ordinary read or event", async () => {
  const read = deferred<UsageSnapshot[]>()
  const input = fixture(read.promise)
  const {result, rerender} = renderHook(({session}: {session: Code0SessionDTO}) => useUsageSnapshots(input.bridge, undefined, session), {
    initialProps: {session: code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})},
  })
  await waitFor(() => expect(input.bridge.onSnapshotUpdated).toHaveBeenCalledOnce())
  rerender({session: code0Session({revision: 3, state: "disconnected", snapshot: null})})
  await act(async () => { read.resolve([...ordinary, oldCode0]); await read.promise })
  act(() => input.emit(oldCode0))
  expect(code0(result.current)?.primaryMetric).toBeFalsy()
  expect(code0(result.current)?.secondaryMetric).toBeFalsy()
  expect(code0(result.current)?.fetchedAt).not.toBe(oldCode0.fetchedAt)
})

it("ordinary Code0 values are not a fallback before any session publication", async () => {
  const input = fixture(Promise.resolve([...ordinary, oldCode0]))
  const {result} = renderHook(() => useUsageSnapshots(input.bridge, undefined, emptyCode0Session))
  await waitFor(() => expect(input.bridge.readSnapshots).toHaveBeenCalledOnce())
  await act(async () => { await Promise.resolve() })
  expect(code0(result.current)?.primaryMetric).toBeFalsy()
})

it("current Code0 appears even when the ordinary list contains only four providers", async () => {
  const input = fixture(Promise.resolve(ordinary))
  const current = code0Session({snapshot: code0Snapshot(8, 3)})
  const {result} = renderHook(() => useUsageSnapshots(input.bridge, undefined, current))
  await act(async () => { await Promise.resolve() })
  expect(code0(result.current)).toEqual(current.snapshot)
  expect(result.current.filter(value => value.providerId === "code0")).toHaveLength(1)
})

it("initialSnapshots cannot supply old Code0 metrics instead of current B", () => {
  const input = fixture(Promise.resolve([]))
  const current = code0Session({snapshot: code0Snapshot(8, 3)})
  const {result} = renderHook(() => useUsageSnapshots(input.bridge, [...ordinary, oldCode0], current))
  expect(code0(result.current)).toEqual(current.snapshot)
  expect(input.bridge.readSnapshots).not.toHaveBeenCalled()
})

it("an ordinary provider event wins that provider's late initial read", async () => {
  const read = deferred<UsageSnapshot[]>()
  const input = fixture(read.promise)
  const {result} = renderHook(() => useUsageSnapshots(input.bridge))
  await waitFor(() => expect(input.bridge.onSnapshotUpdated).toHaveBeenCalledOnce())
  const newer: UsageSnapshot = {...ordinary[0], status: "fresh", usedRatio: 0.42}
  act(() => input.emit(newer))
  await act(async () => { read.resolve(ordinary); await read.promise })
  expect(result.current.find(value => value.providerId === "claude")).toEqual(newer)
})

it("a missing ordinary provider is inserted when its event arrives", async () => {
  const input = fixture(Promise.resolve([ordinary[0]]))
  const {result} = renderHook(() => useUsageSnapshots(input.bridge))
  await act(async () => { await Promise.resolve() })
  const newer: UsageSnapshot = {...ordinary[1], status: "fresh", usedRatio: 0.42}
  act(() => input.emit(newer))
  expect(result.current.find(value => value.providerId === "codex")).toEqual(newer)
})


it("controlled initial snapshots settle across equivalent references, changed ordinary values and Code0 clear", async () => {
  const input = fixture(Promise.resolve([]))
  const current = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
  let renders = 0
  const {result, rerender} = renderHook(({ratio, session}: {ratio: number; session: Code0SessionDTO}) => {
    // Fail a self-sustaining update promptly, without relying on timeout or OOM.
    if (++renders > 16) throw new Error("Controlled initial snapshots did not settle within the render budget")
    const values: UsageSnapshot[] = ordinary.map(value => value.providerId === "claude"
      ? {...value, status: "fresh", usedRatio: ratio} : {...value})
    return useUsageSnapshots(input.bridge, [...values, {...oldCode0}], session)
  }, {initialProps: {ratio: 0.12, session: current}})
  expect(result.current.find(value => value.providerId === "claude")?.usedRatio).toBe(0.12)
  expect(code0(result.current)).toEqual(current.snapshot)

  // Recreate equivalent arrays and elements in the same hook host render.
  rerender({ratio: 0.12, session: current})
  expect(result.current.find(value => value.providerId === "claude")?.usedRatio).toBe(0.12)
  expect(code0(result.current)).toEqual(current.snapshot)

  rerender({ratio: 0.42, session: current})
  expect(result.current.find(value => value.providerId === "claude")?.usedRatio).toBe(0.42)
  expect(code0(result.current)).toEqual(current.snapshot)

  rerender({ratio: 0.42, session: code0Session({revision: 3, state: "disconnected", snapshot: null})})
  await act(async () => { await Promise.resolve() })
  expect(result.current.find(value => value.providerId === "claude")?.usedRatio).toBe(0.42)
  expect(code0(result.current)?.primaryMetric).toBeFalsy()
  expect(code0(result.current)?.secondaryMetric).toBeFalsy()
  expect(code0(result.current)?.fetchedAt).not.toBe(oldCode0.fetchedAt)
  expect(result.current.map(value => value.providerId)).toEqual(["claude", "codex", "deepseek", "gemini", "code0"])
  expect(renders).toBeLessThanOrEqual(16)
  expect(input.bridge.readSnapshots).not.toHaveBeenCalled()
  expect(input.bridge.onSnapshotUpdated).not.toHaveBeenCalled()
})

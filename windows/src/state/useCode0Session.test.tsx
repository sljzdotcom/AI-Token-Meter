import { act, renderHook, waitFor } from "@testing-library/react"
import { expect, it } from "vitest"
import { useCode0Session } from "./useCode0Session"
import { code0BridgeFixture, code0Session, code0Snapshot, deferred } from "../test/code0SessionFixtures"

it("a read before any event can display cached data without authorizing this runtime", async () => {
  const cached = code0Session()
  const fixture = code0BridgeFixture(Promise.resolve(cached))
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await waitFor(() => expect(result.current).toEqual(cached))
  expect(result.current.isAutomaticallySyncing).toBe(false)
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
})

it.each([false, true])("event B or explicit clear wins over late initial A (clear=%s)", async clear => {
  const initial = deferred<unknown>()
  const fixture = code0BridgeFixture(initial.promise)
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await waitFor(() => expect(fixture.bridge.onUpdated).toHaveBeenCalledOnce())
  const b = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
  act(() => fixture.emit(b))
  expect.soft(result.current).toEqual(b)
  const current = clear ? code0Session({revision: 3, state: "disconnected", snapshot: null}) : b
  if (clear) act(() => fixture.emit(current))
  await act(async () => { initial.resolve(code0Session({revision: 1})); await initial.promise })
  expect(result.current).toEqual(current)
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
})

it("a read error keeps an accepted event rather than resetting to disconnected", async () => {
  const initial = deferred<unknown>()
  const fixture = code0BridgeFixture(initial.promise)
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  const b = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
  act(() => fixture.emit(b))
  expect.soft(result.current).toEqual(b)
  await act(async () => { initial.reject(new Error("controlled-read-failure")); await Promise.resolve() })
  expect(result.current).toEqual(b)
})

it("lower revisions and conflicting same-revision publications cannot replace current B", async () => {
  const fixture = code0BridgeFixture(Promise.resolve(code0Session({revision: 1})))
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await act(async () => { await Promise.resolve() })
  const b = code0Session({revision: 4, snapshot: code0Snapshot(8, 3)})
  act(() => fixture.emit(b))
  expect.soft(result.current).toEqual(b)
  act(() => {
    fixture.emit(code0Session({revision: 3, snapshot: null}))
    fixture.emit(code0Session({revision: 4, snapshot: code0Snapshot(99, 4)}))
  })
  expect(result.current).toEqual(b)
})

it("state/failure changes publish independently of unchanged snapshot data", async () => {
  const a = code0Session()
  const fixture = code0BridgeFixture(Promise.resolve(a))
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await act(async () => { await Promise.resolve() })
  const failed = code0Session({revision: 2, failure: "storageFailed", snapshot: a.snapshot})
  act(() => fixture.emit(failed))
  expect(result.current).toEqual(failed)
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
})

it("ordinary account-shaped data cannot overwrite the independent session projection", async () => {
  const b = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
  const fixture = code0BridgeFixture(Promise.resolve(b))
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await act(async () => { await Promise.resolve() })
  expect.soft(result.current).toEqual(b)
  act(() => fixture.emit({providerId: "code0", connectionState: "connected", accountLabel: "discard"}))
  expect(result.current).toEqual(b)
})

it("listener disposal and late messages cannot perform a website action", async () => {
  const fixture = code0BridgeFixture()
  const {unmount} = renderHook(() => useCode0Session(fixture.bridge))
  await act(async () => { await Promise.resolve() })
  unmount()
  fixture.emit(code0Session({revision: 9, state: "active", isAutomaticallySyncing: true}))
  expect(fixture.stop).toHaveBeenCalledOnce()
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
})

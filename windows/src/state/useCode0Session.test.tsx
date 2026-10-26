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


it("accepts revision zero as the first cached publication", async () => {
  const cached = code0Session({revision: 0})
  const fixture = code0BridgeFixture(Promise.resolve(cached))
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await waitFor(() => expect(result.current).toEqual(cached))
})

it("bridge replacement starts cold and ignores the retired runtime's late read", async () => {
  const oldRead = deferred<unknown>()
  const old = code0BridgeFixture(oldRead.promise)
  const nextRead = deferred<unknown>()
  const next = code0BridgeFixture(nextRead.promise)
  const {result, rerender} = renderHook(({bridge}) => useCode0Session(bridge), {initialProps: {bridge: old.bridge}})
  await act(async () => { await Promise.resolve() })
  act(() => old.emit(code0Session({revision: 7})))
  expect(result.current.revision).toBe(7)
  rerender({bridge: next.bridge})
  expect(result.current.snapshot).toBeNull()
  expect(result.current.state).toBe("disconnected")
  const current = code0Session({revision: 0, snapshot: code0Snapshot(8, 3)})
  await act(async () => { nextRead.resolve(current); await nextRead.promise })
  expect(result.current).toEqual(current)
  await act(async () => { oldRead.resolve(code0Session({revision: 99})); await oldRead.promise })
  expect(result.current).toEqual(current)
  expect(old.stop).toHaveBeenCalledOnce()
  expect(next.bridge.perform).not.toHaveBeenCalled()
})

it("an asynchronous listener acknowledgement after unmount is released without starting a read", async () => {
  const registration = deferred<() => void>()
  const fixture = code0BridgeFixture()
  fixture.bridge.onUpdated = () => registration.promise
  const {unmount} = renderHook(() => useCode0Session(fixture.bridge))
  unmount()
  await act(async () => { registration.resolve(fixture.stop); await registration.promise })
  expect(fixture.stop).toHaveBeenCalledOnce()
  expect(fixture.bridge.read).not.toHaveBeenCalled()
  expect(fixture.bridge.perform).not.toHaveBeenCalled()
})

it("an invalid high-revision event cannot poison the next valid publication", async () => {
  const fixture = code0BridgeFixture()
  const {result} = renderHook(() => useCode0Session(fixture.bridge))
  await act(async () => { await Promise.resolve() })
  act(() => fixture.emit({...code0Session({revision: 999}), epoch: "discard"}))
  const current = code0Session({revision: 2, snapshot: code0Snapshot(8, 3)})
  act(() => fixture.emit(current))
  expect(result.current).toEqual(current)
})

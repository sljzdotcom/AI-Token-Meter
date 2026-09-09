import { expect, it } from "vitest"

import type { ProviderId, UsageStatus } from "../state/usage"
import { detailRecovery } from "./serviceRecovery"

it("maps every provider and state to the supported detail recovery", () => {
  const passive: UsageStatus[] = ["fresh", "cached", "refreshing"]
  const actionable: UsageStatus[] = [
    "authenticationRequired",
    "setupRequired",
    "notInstalled",
    "unavailable",
    "unrecognizedOutput",
  ]

  for (const providerId of ["claude", "codex", "deepseek"] satisfies ProviderId[]) {
    for (const status of passive) expect(detailRecovery(providerId, status)).toBeNull()
    for (const status of actionable) expect(detailRecovery(providerId, status)).toBe("services")
  }
  for (const status of [...passive, ...actionable]) {
    expect(detailRecovery("gemini", status)).toBe("geminiControls")
  }
  for (const providerId of ["claude", "codex", "deepseek"] satisfies ProviderId[]) {
    expect(detailRecovery(providerId, "cached", "Cached · sign in required")).toBe("services")
    expect(detailRecovery(providerId, "cached", "Cached · authentication required")).toBe("services")
    expect(detailRecovery(providerId, "cached", "Cached · refresh timed out")).toBeNull()
  }
})

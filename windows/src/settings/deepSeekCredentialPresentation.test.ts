import { expect, it } from "vitest"

import { deepSeekCredentialPresentation } from "./deepSeekCredentialPresentation"

it("uses save copy when no DeepSeek Key exists", () => {
  expect(deepSeekCredentialPresentation(false)).toEqual({
    actionTitle: "Save API Key",
    actionLabel: "Save DeepSeek API Key",
    pendingMessage: "Open the protected Windows prompt to save the API Key.",
    successMessage: "DeepSeek accepted and saved the API Key.",
    failureMessage: "The API Key was not saved. No existing Key was changed.",
  })
})

it("uses replacement copy when a DeepSeek Key already exists", () => {
  expect(deepSeekCredentialPresentation(true)).toEqual({
    actionTitle: "Replace API Key",
    actionLabel: "Replace DeepSeek API Key",
    pendingMessage: "Open the protected Windows prompt to replace the API Key.",
    successMessage: "DeepSeek accepted the replacement API Key.",
    failureMessage: "The replacement was not saved. The existing API Key remains active.",
  })
})


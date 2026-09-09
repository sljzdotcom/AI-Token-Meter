export type DeepSeekCredentialPresentation = {
  actionTitle: string
  actionLabel: string
  pendingMessage: string
  successMessage: string
  failureMessage: string
}

export function deepSeekCredentialPresentation(hasExistingKey: boolean): DeepSeekCredentialPresentation {
  return hasExistingKey ? {
    actionTitle: "Replace API Key",
    actionLabel: "Replace DeepSeek API Key",
    pendingMessage: "Open the protected Windows prompt to replace the API Key.",
    successMessage: "DeepSeek accepted the replacement API Key.",
    failureMessage: "The replacement was not saved. The existing API Key remains active.",
  } : {
    actionTitle: "Save API Key",
    actionLabel: "Save DeepSeek API Key",
    pendingMessage: "Open the protected Windows prompt to save the API Key.",
    successMessage: "DeepSeek accepted and saved the API Key.",
    failureMessage: "The API Key was not saved. No existing Key was changed.",
  }
}


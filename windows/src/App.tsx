import providerContract from "../../contracts/presentation/providers.json"

type ProviderContract = {
  id: "claude" | "codex" | "deepseek"
  displayName: string
  accentColor: string
}

export function App() {
  const providers = providerContract.providers as ProviderContract[]

  return (
    <main className="meter-shell" aria-label="AI Token Meter">
      {providers.map((provider) => (
        <button
          aria-label={`${provider.displayName} usage`}
          className="provider-button"
          key={provider.id}
          style={{ "--provider-accent": provider.accentColor } as React.CSSProperties}
          type="button"
        >
          <span aria-hidden="true" className="provider-mark">
            {provider.id === "claude" ? "✦" : provider.id === "codex" ? "◎" : "◒"}
          </span>
          <span className="provider-status">Unavailable</span>
        </button>
      ))}
    </main>
  )
}

import providerContract from "../../../contracts/presentation/providers.json"
import type { CSSProperties } from "react"

import type { UsageSnapshot } from "../state/usage"
import { ProviderLogo } from "./ProviderLogo"

type UsageRingProps = {
  snapshot: UsageSnapshot
  selected?: boolean
  onActivate: () => void
}

export function UsageRing({ snapshot, selected = false, onActivate }: UsageRingProps) {
  const contract = providerContract.providers.find((provider) => provider.id === snapshot.providerId)
  const hasProgress = snapshot.usedRatio != null && ["fresh", "cached", "refreshing"].includes(snapshot.status)
  const percent = hasProgress ? Math.min(Math.max(snapshot.usedRatio! * 100, 0), 100) : null
  const style = {
    "--provider-accent": contract?.accentColor ?? "#7386FF",
    "--usage-progress": percent == null ? "0deg" : `${percent * 3.6}deg`,
  } as CSSProperties

  return (
    <button
      aria-label={`${snapshot.displayName} usage`}
      aria-pressed={selected}
      className={`usage-ring usage-ring--${snapshot.status}`}
      onClick={onActivate}
      style={style}
      type="button"
    >
      <span
        aria-label={`${snapshot.displayName} usage`}
        aria-valuemax={hasProgress ? 100 : undefined}
        aria-valuemin={hasProgress ? 0 : undefined}
        aria-valuenow={percent == null ? undefined : Number(percent.toFixed(2))}
        className="usage-ring__progress"
        role="progressbar"
      >
        <span className="usage-ring__core">
          <ProviderLogo provider={snapshot.providerId} />
        </span>
      </span>
    </button>
  )
}

import providerContract from "../../../contracts/presentation/providers.json"
import type { CSSProperties } from "react"

import type { UsageSnapshot } from "../state/usage"
import { ProviderLogo } from "./ProviderLogo"
import { t, useLocale } from "../localization"

type UsageRingProps = {
  snapshot: UsageSnapshot
  selected?: boolean
  needsAction?: boolean
  onActivate: () => void
}

export function UsageRing({ snapshot, selected = false, needsAction = false, onActivate }: UsageRingProps) {
  useLocale()
  const contract = providerContract.providers.find((provider) => provider.id === snapshot.providerId)
  const hasProgress = snapshot.usedRatio != null && ["fresh", "cached", "refreshing"].includes(snapshot.status)
  const percent = hasProgress ? Math.min(Math.max(snapshot.usedRatio! * 100, 0), 100) : null
  const style = {
    "--provider-accent": contract?.accentColor ?? "#7386FF",
    "--usage-progress": percent == null ? "0deg" : `${percent * 3.6}deg`,
  } as CSSProperties
  const operation = snapshot.status === "refreshing" ? "refreshing"
    : needsAction || ["authenticationRequired", "setupRequired", "notInstalled"].includes(snapshot.status)
      || ["Cached · sign in required", "Cached · setup required"].includes(snapshot.statusMessage ?? "") ? "waiting" : "idle"

  return (
    <button
      aria-label={t("{name} usage", {name: snapshot.displayName})}
      aria-pressed={selected}
      aria-description={operation === "refreshing" ? t("Refreshing") : operation === "waiting" ? t("Action required") : undefined}
      className={`usage-ring usage-ring--${snapshot.status}`}
      onClick={onActivate}
      style={style}
      type="button"
    >
      <span
        aria-label={t("{name} usage", {name: snapshot.displayName})}
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
      {operation !== "idle" && <span aria-hidden="true" className={`usage-ring__operation usage-ring__operation--${operation}`} />}
      {operation === "waiting" && <span aria-hidden="true" className="usage-ring__action">!</span>}
    </button>
  )
}

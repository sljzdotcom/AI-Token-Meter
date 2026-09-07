import { useState } from "react"
import { t } from "../localization"

const links = [
  { target: "twitter", label: "@MillerPanYue", href: "https://twitter.com/MillerPanYue" },
  { target: "github", label: "GitHub", href: "https://github.com/sljzdotcom/AI-Token-Meter" },
] as const

function BrandMark({target}: {target: BrandLinkTarget}) {
  if (target === "twitter") return <svg aria-hidden="true" className="author-link-icon" viewBox="0 0 24 24"><path d="M4 4l6.2 8.3L4.4 20H7l4.4-5.8 4.3 5.8H20l-6.5-8.7L19 4h-2.6l-4.1 5.4L8.3 4H4zm3 2h1.3L17 18h-1.3L7 6z" /></svg>
  return <svg aria-hidden="true" className="author-link-icon" viewBox="0 0 24 24"><path fillRule="evenodd" d="M12 2a10 10 0 00-3.2 19.5c.5.1.7-.2.7-.5v-1.9c-2.8.6-3.4-1.2-3.4-1.2-.5-1.2-1.1-1.5-1.1-1.5-.9-.6.1-.6.1-.6 1 0 1.6 1.1 1.6 1.1.9 1.6 2.4 1.1 2.9.9.1-.7.4-1.1.7-1.4-2.2-.3-4.6-1.1-4.6-5A3.9 3.9 0 016.8 8.7c-.1-.3-.5-1.3.1-2.7 0 0 .9-.3 2.8 1.1a9.7 9.7 0 015.1 0C16.7 5.7 17.6 6 17.6 6c.6 1.4.2 2.4.1 2.7a3.9 3.9 0 011 2.7c0 3.9-2.4 4.7-4.6 5 .4.3.7.9.7 1.8V21c0 .3.2.6.7.5A10 10 0 0012 2z" clipRule="evenodd" /></svg>
}

export type BrandLinkTarget = (typeof links)[number]["target"]

export function AuthorLinks({onOpen}: {onOpen: (target: BrandLinkTarget) => Promise<void> | void}) {
  const [failed, setFailed] = useState(false)
  return (
    <section className="author-links" aria-label={t("Author links")}>
      <small>{t("Author links")}</small>
      <div>
        {links.map(link => (
          <a key={link.target} href={link.href} onClick={event => {
            event.preventDefault()
            setFailed(false)
            void (async () => {
              try { await onOpen(link.target) }
              catch { setFailed(true) }
            })()
          }}>
            <BrandMark target={link.target} />
            {link.label}
          </a>
        ))}
      </div>
      {failed ? <small role="status">{t("The author link could not be opened.")}</small> : null}
    </section>
  )
}

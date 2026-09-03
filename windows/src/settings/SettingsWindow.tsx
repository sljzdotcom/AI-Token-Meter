import { useState } from "react"
import type { ReactNode } from "react"

const tabs = ["Appearance", "Monitoring", "Services", "About"] as const
const fonts = [
  "System Default",
  "Antonio",
  "DIN Condensed",
  "Alimama FangYuanTi VF",
  "Fira Code",
  "Leigo",
  "Menlo",
  "Alimama DaoLiTi",
]

type SettingsWindowProps = {
  displayFont: string
  onDisplayFontChange: (font: string) => void
}

export function SettingsWindow({ displayFont, onDisplayFontChange }: SettingsWindowProps) {
  const [activeTab, setActiveTab] = useState<(typeof tabs)[number]>("Appearance")
  return (
    <section aria-label="AI Token Meter Settings" className="settings-window settings-window--system-font" role="dialog">
      <header><div><strong>AI Token Meter</strong><small>Private AI usage, at a glance.</small></div></header>
      <nav aria-label="Settings categories" role="tablist">
        {tabs.map((tab) => (
          <button
            aria-selected={activeTab === tab}
            key={tab}
            onClick={() => setActiveTab(tab)}
            role="tab"
            type="button"
          >{tab}</button>
        ))}
      </nav>
      <div className="settings-content">
        {activeTab === "Appearance" ? (
          <>
            <SettingRow label="Display font" hint="Applies to the meter, menu and detail panels. Settings always uses the system font.">
              <select aria-label="Display font" onChange={(event) => onDisplayFontChange(event.target.value)} value={displayFont}>
                {fonts.map((font) => <option key={font}>{font}</option>)}
              </select>
              <button onClick={() => onDisplayFontChange("System Default")} type="button">Restore default font</button>
            </SettingRow>
            <SettingRow label="Screen edge" hint="The meter follows the selected display and stays outside the taskbar.">
              <select aria-label="Screen edge" defaultValue="Right"><option>Right</option><option>Left</option></select>
            </SettingRow>
          </>
        ) : null}
        {activeTab === "Monitoring" ? (
          <SettingRow label="Detail auto-hide" hint="Interaction pauses the countdown.">
            <input aria-label="Detail auto-hide seconds" defaultValue="8" min="1" type="number" /> seconds
          </SettingRow>
        ) : null}
        {activeTab === "Services" ? (
          <div className="service-list">
            <Service name="Claude Code" action="Sign in" />
            <Service name="OpenAI Codex" action="Sign in" />
            <Service name="DeepSeek" action="Replace API Key" />
          </div>
        ) : null}
        {activeTab === "About" ? (
          <div className="about-card"><strong>AI Token Meter</strong><p>Version 0.2.2</p><p>Author · Miller</p><button type="button">Check for Updates</button></div>
        ) : null}
      </div>
    </section>
  )
}

function SettingRow({ label, hint, children }: { label: string; hint: string; children: ReactNode }) {
  return <section className="setting-row"><div><strong>{label}</strong><small>{hint}</small></div><div>{children}</div></section>
}

function Service({ name, action }: { name: string; action: string }) {
  return <article><span><strong>{name}</strong><small>Account status will appear here.</small></span><button type="button">{action}</button></article>
}

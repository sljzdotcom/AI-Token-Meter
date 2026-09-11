type SettingsTabIconProps = {
  tab: "Appearance" | "Floating Strip" | "Monitoring" | "Services" | "About"
}

export function SettingsTabIcon({ tab }: SettingsTabIconProps) {
  const common = {
    "aria-hidden": true,
    "data-settings-tab-icon": tab.toLowerCase().replaceAll(" ", "-"),
    fill: "none",
    height: 16,
    stroke: "currentColor",
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    strokeWidth: 1.5,
    viewBox: "0 0 16 16",
    width: 16,
  }

  switch (tab) {
    case "Appearance":
      return <svg {...common}><path d="M8 2a6 6 0 1 0 0 12h1a1.5 1.5 0 0 0 0-3h-.6a1.4 1.4 0 0 1 0-2.8H10A4 4 0 0 0 8 2Z" /><path d="M4.5 6h.01M6.5 4h.01M11.5 6h.01" /></svg>
    case "Floating Strip":
      return <svg {...common}><rect height="12" rx="3" width="6" x="5" y="2" /><path d="M8 5v.01M8 8v.01M8 11v.01" /></svg>
    case "Monitoring":
      return <svg {...common}><path d="M1.5 8h2l1.25-3 2.5 6 1.5-4 1.5 2H14.5" /></svg>
    case "Services":
      return <svg {...common}><path d="M6 5.5 4.5 4a2.1 2.1 0 0 0-3 3l2 2a2.1 2.1 0 0 0 3 0l1-1M10 10.5l1.5 1.5a2.1 2.1 0 0 0 3-3l-2-2a2.1 2.1 0 0 0-3 0l-1 1M5.5 10.5l5-5" /></svg>
    case "About":
      return <svg {...common}><circle cx="8" cy="8" r="6" /><path d="M8 7.25V11M8 5h.01" /></svg>
  }
}

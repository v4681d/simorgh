import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { applyDirection } from "./i18n"
import { MarkMono, Badge } from "./components"
import Dashboard from "./screens/Dashboard"
import Devices from "./screens/Devices"
import Engines from "./screens/Engines"
import Models from "./screens/Models"
import History from "./screens/History"
import Settings from "./screens/Settings"
import Wizard from "./screens/Wizard"
import { getConfig } from "./api"

const SCREENS = ["dashboard", "devices", "engines", "models", "history", "settings"] as const
export type Screen = (typeof SCREENS)[number]

export default function App() {
  const { t, i18n } = useTranslation()
  const [screen, setScreen] = useState<Screen>("dashboard")
  const [showWizard, setShowWizard] = useState(false)

  useEffect(() => {
    getConfig().then((cfg) => {
      // First run: no devices selected yet -> onboarding wizard (light theme).
      if (!cfg.devices.mic_device && !cfg.devices.loopback_device) setShowWizard(true)
      i18n.changeLanguage(cfg.general.ui_language)
      applyDirection(cfg.general.ui_language)
    }).catch(() => {})
  }, [i18n])

  if (showWizard) return <Wizard onDone={() => setShowWizard(false)} />

  return (
    <div className="flex h-screen">
      {/* Sidebar — dark-gradient mark per brand guide */}
      <nav className="w-52 shrink-0 border-e border-white/20 bg-surface flex flex-col">
        <div className="flex items-center gap-2 px-4 py-4">
          <img src="/brand/simorgh-mark-dark.svg" alt="" className="w-7 h-7"
               onError={(e) => (e.currentTarget.style.display = "none")} />
          <span className="font-bold text-[17px]">Simorgh</span>
        </div>
        <div className="flex flex-col gap-1 px-2">
          {SCREENS.map((s) => (
            <button
              key={s}
              onClick={() => setScreen(s)}
              className={`text-start px-3 py-2 rounded text-[14px] ${
                screen === s ? "bg-raised text-white" : "text-secondary hover:bg-raised/60"
              }`}
            >
              {t(`nav.${s}`)}
            </button>
          ))}
        </div>
        <div className="mt-auto px-4 py-3 flex items-center gap-2 text-secondary">
          <MarkMono size={16} />
          <StatusLine />
        </div>
      </nav>
      <main className="flex-1 overflow-y-auto p-6">
        {screen === "dashboard" && <Dashboard onNavigate={setScreen} />}
        {screen === "devices" && <Devices />}
        {screen === "engines" && <Engines />}
        {screen === "models" && <Models />}
        {screen === "history" && <History />}
        {screen === "settings" && <Settings />}
      </main>
    </div>
  )
}

function StatusLine() {
  // GPU/VRAM surfaced by the backend when local engines are active.
  return <Badge>GPU · auto</Badge>
}

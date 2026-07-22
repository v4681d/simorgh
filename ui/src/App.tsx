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
import Logs from "./screens/Logs"
import Wizard from "./screens/Wizard"
import { getConfig } from "./api"
import { getCurrentWindow } from "@tauri-apps/api/window"

const SCREENS = ["dashboard", "devices", "engines", "models", "history", "settings"] as const
export type Screen = (typeof SCREENS)[number]
type NavScreen = Screen | "logs"

export default function App() {
  const { t, i18n } = useTranslation()
  const [screen, setScreen] = useState<NavScreen>("dashboard")
  const [showWizard, setShowWizard] = useState(false)
  const [devMode, setDevMode] = useState(false)

  useEffect(() => {
    getConfig().then((cfg) => {
      // First run: no devices selected yet -> onboarding wizard (light theme).
      if (!cfg.devices.mic_device && !cfg.devices.loopback_device) setShowWizard(true)
      setDevMode(cfg.general.developer_mode)
      i18n.changeLanguage(cfg.general.ui_language)
      applyDirection(cfg.general.ui_language)
    }).catch(() => {})
  }, [i18n])

  if (showWizard) return <Wizard onDone={() => setShowWizard(false)} />

  return (
    <div className="flex flex-col h-screen">
      <TitleBar />
      <div className="flex flex-1 min-h-0">
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
          {devMode && (
            <button
              onClick={() => setScreen("logs")}
              className={`text-start px-3 py-2 rounded text-[14px] ${
                screen === "logs" ? "bg-raised text-white" : "text-secondary hover:bg-raised/60"
              }`}
            >
              {t("nav.logs")}
            </button>
          )}
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
        {screen === "logs" && devMode && <Logs />}
      </main>
      </div>
    </div>
  )
}

function TitleBar() {
  const win = getCurrentWindow()
  const btn =
    "w-11 h-9 flex items-center justify-center text-secondary hover:bg-white/20 hover:text-white text-[14px]"
  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between h-9 shrink-0 bg-surface border-b border-white/20 select-none"
    >
      <div data-tauri-drag-region className="flex items-center gap-2 px-3">
        <img
          src="/brand/simorgh-mark-dark.svg"
          alt=""
          className="w-4 h-4"
          onError={(e) => (e.currentTarget.style.display = "none")}
        />
        <span className="text-[12px] font-semibold tracking-wide">Simorgh</span>
      </div>
      <div className="flex items-center">
        <button className={btn} onClick={() => win.minimize()} aria-label="Minimize">–</button>
        <button className={btn} onClick={() => win.toggleMaximize()} aria-label="Maximize">▢</button>
        <button
          className="w-11 h-9 flex items-center justify-center text-secondary hover:bg-red-600 hover:text-white text-[14px]"
          onClick={() => win.close()}
          aria-label="Close"
        >
          ✕
        </button>
      </div>
    </div>
  )
}

function StatusLine() {
  // GPU/VRAM surfaced by the backend when local engines are active.
  return <Badge>GPU · auto</Badge>
}

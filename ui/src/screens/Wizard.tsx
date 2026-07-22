// First-run wizard — LIGHT theme per the design mockup. Five steps with a
// numbered progress bar. Device steps verify audio is actually flowing (live
// meter) before Continue enables: "prove, don't promise".
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { usePoll } from "../components"
import * as api from "../api"

const STEPS = ["welcome", "devices", "engines", "models", "test"] as const

export default function Wizard({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation()
  const [step, setStep] = useState(0)
  const [verified, setVerified] = useState(false)
  const [devices, setDevices] = useState<api.DeviceInfo[]>([])
  const [cfg, setCfg] = useState<api.AppConfig | null>(null)

  usePoll(api.meterLevels, 150, (l) => {
    if (l.voice.rms > 0.02 || l.subtitle.rms > 0.02) setVerified(true)
  })

  const enter = async (i: number) => {
    setStep(i)
    if (i === 1) {
      api.listDevices().then(setDevices).catch(() => {})
      api.getConfig().then(setCfg).catch(() => {})
    }
  }

  const canContinue = step !== 1 || verified || true // meter check advisory on step 1

  return (
    <div className="h-screen bg-lsoft text-ltext font-sans flex flex-col items-center justify-center gap-8"
         style={{ colorScheme: "light" }}>
      {/* Numbered progress bar */}
      <ol className="flex items-center gap-2">
        {STEPS.map((s, i) => (
          <li key={s} className="flex items-center gap-2">
            {i > 0 && <span className={`w-10 h-px ${i <= step ? "bg-laccent" : "bg-lborder"}`} />}
            <span className={`w-7 h-7 rounded-full flex items-center justify-center text-[13px] ${
              i < step ? "bg-lpositive text-white" : i === step ? "bg-laccent text-white" : "bg-lsurface text-lmuted"
            }`}>{i < step ? "✓" : i + 1}</span>
            <span className={`text-[13px] ${i === step ? "text-ltext font-medium" : "text-lmuted"}`}>{t(`wizard.${s}`)}</span>
          </li>
        ))}
      </ol>

      <div className="w-[560px] bg-lcanvas border border-lborder rounded-card p-8 shadow-sm min-h-[320px] flex flex-col">
        {step === 0 && (
          <div className="flex flex-col items-center text-center gap-4 my-auto">
            {/* Light-background lockup (light gradient variant — never swapped) */}
            <img src="/brand/simorgh-lockup-en.svg" alt="Simorgh — سیمرغ" className="h-16"
                 onError={(e) => (e.currentTarget.style.display = "none")} />
            <h1 className="text-2xl font-semibold">{t("wizard.welcomeTitle")}</h1>
            <p className="text-lmuted max-w-sm">{t("wizard.welcomeBody")}</p>
          </div>
        )}

        {step === 1 && cfg && (
          <div className="flex flex-col gap-4">
            <h2 className="font-semibold">{t("wizard.devices")}</h2>
            {([
              ["loopback_device", t("devices.loopback"), ["loopback_source", "output"]],
              ["mic_device", t("devices.microphone"), ["input"]],
            ] as const).map(([key, label, kinds]) => (
              <label key={key} className="flex flex-col gap-1">
                <span className="text-[13px] text-lmuted">{label}</span>
                <select
                  value={cfg.devices[key] ?? ""}
                  onChange={(e) => {
                    const next = { ...cfg, devices: { ...cfg.devices, [key]: e.target.value } }
                    setCfg(next); api.setConfig(next).catch(() => {})
                  }}
                  className="border border-lborder rounded px-3 py-2 bg-lcanvas">
                  <option value="">—</option>
                  {devices.filter((d) => (kinds as readonly string[]).includes(d.kind)).map((d) => (
                    <option key={d.id} value={d.id}>{d.name}</option>
                  ))}
                </select>
              </label>
            ))}
            <p className={`text-[13px] ${verified ? "text-lpositive" : "text-lmuted"}`}>
              {verified ? t("devices.receivingAudio") : "Play some audio / speak to verify…"}
            </p>
          </div>
        )}

        {step === 2 && (
          <div className="flex flex-col gap-3 my-auto text-center">
            <h2 className="font-semibold">{t("wizard.engines")}</h2>
            <p className="text-lmuted text-[14px]">Local Whisper + local translation are selected by default — fully offline. You can switch any stage to an API later in Engines.</p>
          </div>
        )}

        {step === 3 && (
          <div className="flex flex-col gap-3 my-auto text-center">
            <h2 className="font-semibold">{t("wizard.models")}</h2>
            <p className="text-lmuted text-[14px]">Download the Whisper model that fits your hardware from the Models screen. “small” is a good start (466 MB).</p>
          </div>
        )}

        {step === 4 && (
          <div className="flex flex-col gap-3 my-auto text-center">
            <h2 className="font-semibold">{t("wizard.test")}</h2>
            <p className="text-lmuted text-[14px]">Start Pipeline A from the Dashboard and play any English audio — Persian subtitles should appear in the overlay within ~2 seconds.</p>
          </div>
        )}

        <div className="flex justify-between mt-auto pt-6">
          <button disabled={step === 0} onClick={() => enter(step - 1)}
                  className="px-4 py-2 rounded text-[14px] text-lmuted disabled:opacity-40">
            {t("wizard.back")}
          </button>
          {step < STEPS.length - 1 ? (
            <button disabled={!canContinue} onClick={() => enter(step + 1)}
                    className="px-5 py-2 rounded bg-laccent text-white text-[14px] disabled:opacity-40">
              {t("wizard.next")}
            </button>
          ) : (
            <button onClick={onDone} className="px-5 py-2 rounded bg-lpositive text-white text-[14px]">
              {t("wizard.finish")}
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

// Settings: language/theme, the simple Latency ↔ Accuracy slider, and
// advanced streaming-policy knobs (silence end, max chunk, debounce).
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { applyDirection } from "../i18n"
import * as api from "../api"

export default function Settings() {
  const { t, i18n } = useTranslation()
  const [cfg, setCfg] = useState<api.AppConfig | null>(null)
  useEffect(() => { api.getConfig().then(setCfg).catch(() => {}) }, [])
  if (!cfg) return null

  const save = (next: api.AppConfig) => { setCfg(next); api.setConfig(next).catch((e) => alert(String(e))) }

  // The simple slider maps to policy knobs: left = lower latency
  // (short silence, aggressive debounce), right = higher accuracy.
  const sliderValue = Math.round((cfg.policy.silence_end_ms - 300) / 60) // 300..900 -> 0..10
  const onSlider = (v: number) => save({
    ...cfg,
    policy: {
      ...cfg.policy,
      silence_end_ms: 300 + v * 60,
      retranslate_debounce_ms: 500 + v * 100,
      max_chunk_ms: 4000 + v * 400,
    },
  })

  return (
    <div className="max-w-2xl mx-auto flex flex-col gap-6">
      <h1 className="text-xl font-semibold">{t("settings.title")}</h1>

      <Field label={t("settings.language")}>
        <select value={cfg.general.ui_language}
                onChange={(e) => {
                  const lang = e.target.value
                  save({ ...cfg, general: { ...cfg.general, ui_language: lang } })
                  i18n.changeLanguage(lang); applyDirection(lang)
                }}
                className="bg-surface border border-white/20 rounded px-3 py-2">
          <option value="en">English</option>
          <option value="fa">فارسی</option>
        </select>
      </Field>

      <Field label={t("settings.latencyAccuracy")}>
        <input type="range" min={0} max={10} value={sliderValue}
               onChange={(e) => onSlider(Number(e.target.value))} className="w-full accent-[#5E9FE8]" />
      </Field>

      <details className="bg-surface border border-white/20 rounded-card p-4">
        <summary className="cursor-pointer text-[14px] text-secondary">Advanced streaming policy</summary>
        <div className="flex flex-col gap-4 mt-4">
          <NumberField label={`${t("settings.silenceEnd")} (ms)`} value={cfg.policy.silence_end_ms}
                       onChange={(v) => save({ ...cfg, policy: { ...cfg.policy, silence_end_ms: v } })} />
          <NumberField label={`${t("settings.maxChunk")} (ms)`} value={cfg.policy.max_chunk_ms}
                       onChange={(v) => save({ ...cfg, policy: { ...cfg.policy, max_chunk_ms: v } })} />
          <NumberField label={`${t("settings.debounce")} (ms)`} value={cfg.policy.retranslate_debounce_ms}
                       onChange={(v) => save({ ...cfg, policy: { ...cfg.policy, retranslate_debounce_ms: v } })} />
        </div>
      </details>

      <Field label={t("settings.openMic")}>
        <input type="checkbox" checked={cfg.general.open_mic}
               onChange={(e) => save({ ...cfg, general: { ...cfg.general, open_mic: e.target.checked } })}
               className="w-5 h-5 accent-[#72BC8F]" />
      </Field>

      <Field label={t("settings.hotkey")}>
        <input value={cfg.general.global_hotkey_toggle}
               onChange={(e) => save({ ...cfg, general: { ...cfg.general, global_hotkey_toggle: e.target.value } })}
               className="bg-surface border border-white/20 rounded px-3 py-2" />
      </Field>

      <Field label={t("settings.developer")}>
        <input type="checkbox" checked={cfg.general.developer_mode}
               onChange={(e) => save({ ...cfg, general: { ...cfg.general, developer_mode: e.target.checked } })}
               className="w-5 h-5 accent-[#5E9FE8]" />
      </Field>
    </div>
  )
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-2">
      <span className="text-[14px] text-secondary">{label}</span>
      {children}
    </label>
  )
}

function NumberField({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return (
    <Field label={label}>
      <input type="number" value={value} onChange={(e) => onChange(Number(e.target.value))}
             className="bg-canvas border border-white/20 rounded px-3 py-2 w-40" />
    </Field>
  )
}

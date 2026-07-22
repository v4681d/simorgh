// Device pickers with hot-plug refresh and live meters under each dropdown
// ("prove, don't promise"). Virtual mic setup status with wizard guidance.
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { VuMeter, Badge, usePoll } from "../components"
import * as api from "../api"

export default function Devices() {
  const { t } = useTranslation()
  const [devices, setDevices] = useState<api.DeviceInfo[]>([])
  const [cfg, setCfg] = useState<api.AppConfig | null>(null)
  const [vmic, setVmic] = useState<{ status: string; device?: string; instructions?: string } | null>(null)
  const [levels, setLevels] = useState({ subtitle: 0, voice: 0 })

  const refresh = () => api.listDevices().then(setDevices).catch(() => {})
  useEffect(() => {
    refresh()
    api.getConfig().then(setCfg).catch(() => {})
    api.ensureVirtualMic().then(setVmic).catch(() => {})
    const id = setInterval(refresh, 3000) // hot-plug refresh
    return () => clearInterval(id)
  }, [])
  usePoll(api.meterLevels, 100, (l) => setLevels({ subtitle: l.subtitle.rms, voice: l.voice.rms }))

  const save = (patch: Partial<api.AppConfig["devices"]>) => {
    if (!cfg) return
    const next = { ...cfg, devices: { ...cfg.devices, ...patch } }
    setCfg(next)
    api.setConfig(next).catch((e) => alert(String(e)))
  }

  const options = (kinds: api.DeviceInfo["kind"][]) =>
    devices.filter((d) => kinds.includes(d.kind)).map((d) => (
      <option key={d.id} value={d.id}>{d.name}{d.is_default ? " ✓" : ""}</option>
    ))

  if (!cfg) return null
  return (
    <div className="max-w-2xl mx-auto flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t("devices.title")}</h1>
        <button onClick={refresh} className="px-3 py-1.5 rounded bg-raised text-[14px] hover:bg-white/20">
          {t("devices.refresh")}
        </button>
      </div>

      <Picker label={t("devices.loopback")} value={cfg.devices.loopback_device ?? ""}
              onChange={(v) => save({ loopback_device: v })} level={levels.subtitle}>
        {options(["loopback_source", "output"])}
      </Picker>

      <Picker label={t("devices.microphone")} value={cfg.devices.mic_device ?? ""}
              onChange={(v) => save({ mic_device: v })} level={levels.voice}>
        {options(["input"])}
      </Picker>

      <Picker label={t("devices.virtualMic")} value={cfg.devices.virtual_mic_sink ?? ""}
              onChange={(v) => save({ virtual_mic_sink: v })} level={0}>
        {options(["virtual_mic_sink", "output"])}
      </Picker>

      {vmic && (
        <div className="bg-surface border border-white/20 rounded-card p-4 flex flex-col gap-2">
          <Badge tone={vmic.status === "ready" ? "positive" : "attention"}>
            {vmic.status === "ready" ? `Virtual mic ready: ${vmic.device}` : vmic.status}
          </Badge>
          {vmic.instructions && <p className="text-secondary text-[14px]">{vmic.instructions}</p>}
        </div>
      )}
    </div>
  )
}

function Picker({ label, value, onChange, level, children }: {
  label: string; value: string; onChange: (v: string) => void; level: number; children: React.ReactNode
}) {
  const { t } = useTranslation()
  return (
    <label className="flex flex-col gap-2">
      <span className="text-[14px] text-secondary">{label}</span>
      <select value={value} onChange={(e) => onChange(e.target.value)}
              className="bg-surface border border-white/20 rounded px-3 py-2">
        <option value="">—</option>
        {children}
      </select>
      <div className="flex items-center gap-3">
        <VuMeter level={level} />
        {level > 0.01 && <span className="text-positive text-[12px]">{t("devices.receivingAudio")}</span>}
      </div>
    </label>
  )
}

import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Badge, ChipPath, Toggle, VuMeter, usePoll } from "../components"
import type { Screen } from "../App"
import * as api from "../api"
import { useEffect } from "react"

interface Row {
  id: string
  at: string
  source: string
  translated: string
  partial: boolean
}

export default function Dashboard({ onNavigate }: { onNavigate: (s: Screen) => void }) {
  const { t } = useTranslation()
  const [running, setRunning] = useState({ subtitle: false, voice: false })
  const [levels, setLevels] = useState({ subtitle: 0, voice: 0 })
  const [latency, setLatency] = useState<{ per_stage_ms: Record<string, number>; end_to_end_ms: number }>({ per_stage_ms: {}, end_to_end_ms: 0 })
  const [rows, setRows] = useState<Row[]>([])

  usePoll(api.meterLevels, 100, (l) => setLevels({ subtitle: l.subtitle.rms, voice: l.voice.rms }))
  usePoll(api.latencyReport, 1000, setLatency)
  usePoll(api.pipelineStatus, 1000, (s) =>
    setRunning({ subtitle: s.subtitle === "running", voice: s.voice === "running" }))

  useEffect(() => {
    const un = api.onPipelineEvent((ev) => {
      if (ev.type === "translation") {
        setRows((rows) => {
          const at = new Date().toLocaleTimeString()
          const next = rows.filter((r) => r.id !== ev.segment_id || !r.partial)
          // Partial rows are replaced in place; finals are appended and locked.
          const without = next.filter((r) => r.id !== ev.segment_id)
          return [...without, { id: ev.segment_id, at, source: ev.source_text, translated: ev.translated_text, partial: !ev.is_final }].slice(-50)
        })
      }
    })
    return () => { un.then((f) => f()) }
  }, [])

  const toggle = async (kind: api.PipelineKind, on: boolean) => {
    try {
      if (on) await api.startPipeline(kind)
      else await api.stopPipeline(kind)
      setRunning((r) => ({ ...r, [kind]: on }))
    } catch (e) {
      alert(String(e)) // error surfacing with actionable fix text from backend
    }
  }

  const stt = latency.per_stage_ms["stt"] ?? 0
  const mt = latency.per_stage_ms["translate"] ?? 0

  return (
    <div className="max-w-4xl mx-auto flex flex-col gap-6">
      {/* Pipeline A — Listen & subtitle */}
      <section className="bg-surface border border-white/20 rounded-card p-6 flex flex-col gap-4">
        <header className="flex items-center justify-between">
          <h2 className="font-semibold">{t("dashboard.pipelineA")}</h2>
          <div className="flex items-center gap-3">
            <Badge tone={running.subtitle ? "positive" : "neutral"}>
              {running.subtitle ? t("dashboard.running") : t("dashboard.stopped")}
            </Badge>
            <Toggle checked={running.subtitle} onChange={(v) => toggle("subtitle", v)} label={t("dashboard.pipelineA")} />
          </div>
        </header>
        <ChipPath items={[
          { label: "🔊 " + t("devices.loopback"), onClick: () => onNavigate("devices") },
          { label: "Whisper (local)", onClick: () => onNavigate("engines") },
          { label: "فارسی", lang: "fa", onClick: () => onNavigate("engines") },
          { label: "Overlay", onClick: () => onNavigate("settings") },
        ]} />
        <div className="flex items-center justify-between">
          <VuMeter level={levels.subtitle} />
          <div className="flex gap-2">
            <Badge>STT {stt}ms</Badge>
            <Badge>MT {mt}ms</Badge>
            <Badge tone={latency.end_to_end_ms > 2000 ? "attention" : "positive"}>
              {latency.end_to_end_ms}ms {t("dashboard.endToEnd")}
            </Badge>
          </div>
        </div>
      </section>

      {/* Pipeline B — Speak & translate */}
      <section className="bg-surface border border-white/20 rounded-card p-6 flex flex-col gap-4">
        <header className="flex items-center justify-between">
          <h2 className="font-semibold">{t("dashboard.pipelineB")}</h2>
          <div className="flex items-center gap-3">
            <Badge>{t("dashboard.pushToTalk")}: Ctrl+Space</Badge>
            <Toggle checked={running.voice} onChange={(v) => toggle("voice", v)} label={t("dashboard.pipelineB")} />
          </div>
        </header>
        <ChipPath items={[
          { label: "🎤 " + t("devices.microphone"), onClick: () => onNavigate("devices") },
          { label: "STT", onClick: () => onNavigate("engines") },
          { label: "English", onClick: () => onNavigate("engines") },
          { label: "TTS", onClick: () => onNavigate("engines") },
          { label: "🎙️ " + t("devices.virtualMic"), onClick: () => onNavigate("devices") },
        ]} />
        <div className="flex items-center justify-between">
          <VuMeter level={levels.voice} />
          <Badge tone="neutral">~2.5–3.5s honest budget</Badge>
        </div>
      </section>

      {/* Live transcript strip */}
      <section className="bg-surface border border-white/20 rounded-card p-4">
        <h3 className="text-secondary text-[14px] mb-2">{t("dashboard.liveTranscript")}</h3>
        <div className="flex flex-col gap-1 max-h-64 overflow-y-auto">
          {rows.map((r) => (
            <div key={r.id + (r.partial ? "-p" : "-f")} className={`grid grid-cols-[80px_1fr_1fr] gap-3 text-[14px] ${r.partial ? "partial" : ""}`}>
              <span className="text-secondary text-[12px]">{r.at}</span>
              <span>{r.source}</span>
              <span lang="fa" className="text-end">{r.translated}</span>
            </div>
          ))}
          {rows.length === 0 && <p className="text-secondary text-[14px]">—</p>}
        </div>
      </section>
    </div>
  )
}

// Model manager: download with live progress, checksums, resume, disk usage.
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { listen } from "@tauri-apps/api/event"
import { Badge } from "../components"
import * as api from "../api"

// Known model download sources (verified checksums maintained in releases).
const SOURCES: Record<string, string> = {
  "ggml-tiny.bin": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
  "ggml-base.bin": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
  "ggml-small.bin": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
  "ggml-medium.bin": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
  "ggml-large-v3.bin": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
  "ggml-large-v3-q5_0.bin": "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin",
  "en_US-lessac-medium.onnx": "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx",
  "silero_vad.onnx": "https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.onnx",
}

export default function Models() {
  const { t } = useTranslation()
  const [models, setModels] = useState<Awaited<ReturnType<typeof api.listModels>>>([])
  const [progress, setProgress] = useState<Record<string, { done: number; total: number }>>({})

  const refresh = () => api.listModels().then(setModels).catch(() => {})
  useEffect(() => {
    refresh()
    const un = listen<{ name: string; done: number; total: number }>("model-download-progress", (e) => {
      setProgress((p) => ({ ...p, [e.payload.name]: { done: e.payload.done, total: e.payload.total } }))
    })
    return () => { un.then((f) => f()) }
  }, [])

  const download = async (name: string) => {
    try {
      await api.downloadModel(name, SOURCES[name])
      setProgress((p) => { const q = { ...p }; delete q[name]; return q })
      refresh()
    } catch (e) { alert(String(e)) }
  }

  const totalBytes = models.reduce((s, m) => s + (m.size_bytes ?? 0), 0)

  return (
    <div className="max-w-2xl mx-auto flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t("models.title")}</h1>
        <Badge>{t("models.diskUsage")}: {(totalBytes / 1e9).toFixed(2)} GB</Badge>
      </div>
      {models.map((m) => {
        const p = progress[m.name]
        return (
          <div key={m.name} className="bg-surface border border-white/20 rounded-card p-4 flex items-center gap-4">
            <div className="flex-1">
              <p className="text-[14px]">{m.name}</p>
              <p className="text-secondary text-[12px] uppercase">{m.kind}</p>
              {p && p.total > 0 && (
                <div className="mt-2 h-1.5 bg-raised rounded overflow-hidden">
                  <div className="h-full bg-accent transition-all" style={{ width: `${(p.done / p.total) * 100}%` }} />
                </div>
              )}
            </div>
            {m.downloaded ? (
              <>
                <Badge tone="positive">{((m.size_bytes ?? 0) / 1e6).toFixed(0)} MB</Badge>
                <button onClick={() => api.deleteModel(m.name).then(refresh)}
                        className="px-3 py-1.5 rounded bg-raised text-danger text-[14px] hover:bg-white/20">
                  {t("models.delete")}
                </button>
              </>
            ) : (
              <button onClick={() => download(m.name)} disabled={!!p}
                      className="px-3 py-1.5 rounded bg-accent text-[14px] disabled:opacity-50">
                {p ? `${((p.done / Math.max(1, p.total)) * 100).toFixed(0)}%` : t("models.download")}
              </button>
            )}
          </div>
        )
      })}
    </div>
  )
}

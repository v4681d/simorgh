// Engine picker — three cards with identical anatomy (STT / Translation / TTS):
// uppercase stage label, Local↔API segmented switch, model dropdown, status
// badges, and API fields. Applying a change swaps the engine live (the backend
// rebuilds the pipeline stage without an app restart).
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import { Badge, Segmented } from "../components"
import * as api from "../api"

type StageKey = "stt_subtitle" | "translator_subtitle" | "tts"

const CATALOG: Record<StageKey, { local: { engine: string; models: string[] }; apiEngines: { engine: string; label: string; streaming?: boolean }[] }> = {
  stt_subtitle: {
    local: { engine: "whisper-local", models: ["tiny", "base", "small", "medium", "large-v3", "large-v3-q5_0"] },
    apiEngines: [
      { engine: "openai-stt", label: "OpenAI-compatible" },
      { engine: "deepgram", label: "Deepgram", streaming: true },
      { engine: "custom", label: "Custom WS/HTTP", streaming: true },
    ],
  },
  translator_subtitle: {
    local: { engine: "openai-compat", models: ["llama.cpp / Ollama / LM Studio"] },
    apiEngines: [
      { engine: "deepl", label: "DeepL" },
      { engine: "google", label: "Google Translate" },
      { engine: "openai-compat", label: "OpenAI-compatible (cloud)" },
    ],
  },
  tts: {
    local: { engine: "piper", models: ["en_US-lessac-medium", "kokoro"] },
    apiEngines: [
      { engine: "openai-tts", label: "OpenAI-compatible", streaming: true },
      { engine: "elevenlabs", label: "ElevenLabs", streaming: true },
    ],
  },
}

const MODEL_BADGES: Record<string, string> = {
  "tiny": "CPU · 75 MB", "base": "CPU · 142 MB", "small": "GPU · 466 MB",
  "medium": "GPU · 1.5 GB", "large-v3": "GPU · 3 GB", "large-v3-q5_0": "GPU · 1.1 GB",
}

export default function Engines() {
  const { t } = useTranslation()
  const [cfg, setCfg] = useState<api.AppConfig | null>(null)
  useEffect(() => { api.getConfig().then(setCfg).catch(() => {}) }, [])
  if (!cfg) return null

  const stages: { key: StageKey; title: string }[] = [
    { key: "stt_subtitle", title: t("engines.stt") },
    { key: "translator_subtitle", title: t("engines.translation") },
    { key: "tts", title: t("engines.tts") },
  ]

  const update = (key: StageKey, choice: api.EngineChoice) => {
    const next = { ...cfg, engines: { ...cfg.engines, [key]: choice } }
    setCfg(next)
    api.setConfig(next).catch((e) => alert(String(e)))
  }

  return (
    <div className="max-w-3xl mx-auto flex flex-col gap-6">
      {stages.map(({ key, title }) => (
        <EngineCard key={key} stage={key} title={title}
                    choice={cfg.engines[key]} onChange={(c) => update(key, c)} />
      ))}
    </div>
  )
}

function EngineCard({ stage, title, choice, onChange }: {
  stage: StageKey; title: string; choice: api.EngineChoice; onChange: (c: api.EngineChoice) => void
}) {
  const { t } = useTranslation()
  const cat = CATALOG[stage]
  // Only an explicit localhost base_url counts as LOCAL (Ollama/llama.cpp).
  // Empty/remote base_url = API, so the user can reach the key field.
  const isLocalUrl = (url?: string | null) =>
    !!url && (url.includes("localhost") || url.includes("127.0.0.1"))
  const isApi = cat.apiEngines.some((a) => a.engine === choice.engine)
    && (choice.engine !== cat.local.engine || !isLocalUrl(choice.base_url))
  const mode = isApi ? t("engines.api") : t("engines.local")
  const [apiKey, setApiKeyLocal] = useState("")
  const [testResult, setTestResult] = useState<string | null>(null)

  const setMode = (m: string) => {
    if (m === t("engines.local")) {
      // For the OpenAI-compatible local engine, pin base_url to localhost so
      // it's recognized as LOCAL (Ollama/llama.cpp default port).
      const base_url = cat.local.engine === "openai-compat" ? "http://localhost:11434/v1" : choice.base_url
      onChange({ ...choice, engine: cat.local.engine, model: cat.local.models[0], base_url })
    } else {
      onChange({ ...choice, engine: cat.apiEngines[0].engine, model: null })
    }
  }

  const streaming = cat.apiEngines.find((a) => a.engine === choice.engine)?.streaming

  return (
    <section className="bg-surface border border-white/20 rounded-card p-6 flex flex-col gap-4">
      <header className="flex items-center justify-between">
        <span className="uppercase tracking-wider text-[12px] text-secondary">{title}</span>
        <Segmented options={[t("engines.local"), t("engines.api")]} value={mode} onChange={setMode} />
      </header>

      {mode === t("engines.local") ? (
        <div className="flex items-center gap-3">
          <select value={choice.model ?? ""} onChange={(e) => onChange({ ...choice, model: e.target.value })}
                  className="bg-canvas border border-white/20 rounded px-3 py-2 flex-1">
            {cat.local.models.map((m) => <option key={m} value={m}>{m}</option>)}
          </select>
          {choice.model && MODEL_BADGES[choice.model] && <Badge>{MODEL_BADGES[choice.model]}</Badge>}
          <Badge tone="positive">{t("engines.offline")}</Badge>
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-3">
            <select value={choice.engine} onChange={(e) => onChange({ ...choice, engine: e.target.value })}
                    className="bg-canvas border border-white/20 rounded px-3 py-2 flex-1">
              {cat.apiEngines.map((a) => <option key={a.engine} value={a.engine}>{a.label}</option>)}
            </select>
            {streaming && <Badge tone="positive">{t("engines.streaming")}</Badge>}
            <Badge>~$0.02/hr</Badge>
          </div>
          <input placeholder={t("engines.baseUrl")} value={choice.base_url ?? ""}
                 onChange={(e) => onChange({ ...choice, base_url: e.target.value || null })}
                 className="bg-canvas border border-white/20 rounded px-3 py-2" />
          <input placeholder={t("engines.model")} value={choice.model ?? ""}
                 onChange={(e) => onChange({ ...choice, model: e.target.value || null })}
                 className="bg-canvas border border-white/20 rounded px-3 py-2" />
          <div className="flex gap-2">
            <input placeholder={t("engines.apiKey")} type="password" value={apiKey}
                   onChange={(e) => setApiKeyLocal(e.target.value)}
                   className="bg-canvas border border-white/20 rounded px-3 py-2 flex-1" />
            <button className="px-3 rounded bg-raised hover:bg-white/20 text-[14px]"
                    onClick={() => { api.setApiKey(choice.api_key_ref ?? choice.engine, apiKey); setApiKeyLocal("") }}>
              Save
            </button>
          </div>
          <input placeholder={t("engines.proxy")} value={choice.proxy ?? ""}
                 onChange={(e) => onChange({ ...choice, proxy: e.target.value || null })}
                 className="bg-canvas border border-white/20 rounded px-3 py-2" />
          <div className="flex items-center gap-3">
            <button
              className="px-3 rounded bg-raised hover:bg-white/20 text-[14px]"
              onClick={async () => {
                setTestResult("…")
                try {
                  const r = await api.testEngine(stage.replace("_subtitle", ""))
                  setTestResult(r)
                } catch (e) {
                  setTestResult("✗ " + String(e))
                }
              }}
            >
              {t("engines.test")}
            </button>
            {testResult && <span className="text-[13px] text-secondary">{testResult}</span>}
          </div>
        </div>
      )}
    </section>
  )
}

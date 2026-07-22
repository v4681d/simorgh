// Logs: developer-mode view of the rolling app log (tail), auto-refreshing.
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import * as api from "../api"

export default function Logs() {
  const { t } = useTranslation()
  const [text, setText] = useState("")
  const [auto, setAuto] = useState(true)
  const [err, setErr] = useState<string | null>(null)

  const refresh = () =>
    api.getLogs(800).then(setText).catch((e) => setErr(String(e)))

  useEffect(() => {
    refresh()
    if (!auto) return
    const id = setInterval(refresh, 2000)
    return () => clearInterval(id)
  }, [auto])

  return (
    <div className="max-w-5xl mx-auto flex flex-col gap-4 h-full">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t("logs.title")}</h1>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-[13px] text-secondary">
            <input
              type="checkbox"
              checked={auto}
              onChange={(e) => setAuto(e.target.checked)}
              className="accent-[#5E9FE8]"
            />
            {t("logs.autoRefresh")}
          </label>
          <button
            onClick={refresh}
            className="px-3 py-1.5 rounded bg-raised hover:bg-white/20 text-[13px]"
          >
            {t("logs.refresh")}
          </button>
          <button
            onClick={() => navigator.clipboard.writeText(text)}
            className="px-3 py-1.5 rounded bg-raised hover:bg-white/20 text-[13px]"
          >
            {t("logs.copy")}
          </button>
        </div>
      </div>
      {err && <div className="text-red-400 text-[13px]">{err}</div>}
      <pre className="bg-canvas border border-white/20 rounded-card p-3 overflow-auto flex-1 text-[12px] leading-relaxed font-mono whitespace-pre-wrap">
        {text || t("logs.empty")}
      </pre>
      <div className="text-[12px] text-secondary">
        {text ? text.split("\n").length : 0} {t("logs.lines")}
      </div>
    </div>
  )
}

// Subtitle history with TXT / SRT export.
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import * as api from "../api"

interface Row { at: string; source: string; translated: string }

export default function History() {
  const { t } = useTranslation()
  const [rows, setRows] = useState<Row[]>([])

  useEffect(() => {
    const un = api.onPipelineEvent((ev) => {
      if (ev.type === "translation" && ev.is_final) {
        setRows((r) => [...r, { at: new Date().toLocaleTimeString(), source: ev.source_text, translated: ev.translated_text }])
      }
    })
    return () => { un.then((f) => f()) }
  }, [])

  const download = async (format: "txt" | "srt") => {
    const content = await api.exportHistory(format)
    const blob = new Blob([content], { type: "text/plain;charset=utf-8" })
    const a = document.createElement("a")
    a.href = URL.createObjectURL(blob)
    a.download = `simorgh-subtitles.${format}`
    a.click()
    URL.revokeObjectURL(a.href)
  }

  return (
    <div className="max-w-3xl mx-auto flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t("history.title")}</h1>
        <div className="flex gap-2">
          <button onClick={() => download("txt")} className="px-3 py-1.5 rounded bg-raised text-[14px] hover:bg-white/20">{t("history.exportTxt")}</button>
          <button onClick={() => download("srt")} className="px-3 py-1.5 rounded bg-raised text-[14px] hover:bg-white/20">{t("history.exportSrt")}</button>
        </div>
      </div>
      <div className="bg-surface border border-white/20 rounded-card divide-y divide-white/10">
        {rows.map((r, i) => (
          <div key={i} className="grid grid-cols-[90px_1fr_1fr] gap-3 p-3 text-[14px]">
            <span className="text-secondary text-[12px]">{r.at}</span>
            <span>{r.source}</span>
            <span lang="fa" className="text-end">{r.translated}</span>
          </div>
        ))}
        {rows.length === 0 && <p className="p-4 text-secondary text-[14px]">—</p>}
      </div>
    </div>
  )
}

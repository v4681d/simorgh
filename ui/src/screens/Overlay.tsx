// Subtitle overlay — frameless, always-on-top, click-through (toggleable),
// draggable, translucent. Partial→final: provisional words at 50% opacity
// that solidify when finalized; final text is NEVER rewritten.
import { useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { getCurrentWindow } from "@tauri-apps/api/window"
import * as api from "../api"

export default function Overlay() {
  const { t } = useTranslation()
  const [source, setSource] = useState("")
  const [finalText, setFinalText] = useState("")
  const [partialText, setPartialText] = useState("")
  const [fontSize, setFontSize] = useState(19)
  const [behind, setBehind] = useState(0)
  const lastEventAt = useRef(Date.now())

  useEffect(() => {
    document.body.classList.add("overlay")
    api.getConfig().then((c) => setFontSize(c.subtitle.font_size)).catch(() => {})
    const un = api.onPipelineEvent((ev) => {
      if (ev.type !== "translation") return
      lastEventAt.current = Date.now()
      setSource(ev.source_text)
      if (ev.is_final) {
        // Lock the segment: append to final, clear provisional.
        setFinalText(ev.translated_text)
        setPartialText("")
      } else {
        setPartialText(ev.translated_text)
      }
    })
    const id = setInterval(() => setBehind((Date.now() - lastEventAt.current) / 1000), 500)
    return () => { un.then((f) => f()); clearInterval(id) }
  }, [])

  // Scroll to resize font; persisted via config.
  const onWheel = (e: React.WheelEvent) => {
    setFontSize((s) => {
      const next = Math.min(48, Math.max(14, s + (e.deltaY < 0 ? 1 : -1)))
      api.getConfig().then((c) => api.setConfig({ ...c, subtitle: { ...c.subtitle, font_size: next } })).catch(() => {})
      return next
    })
  }

  return (
    <div
      onWheel={onWheel}
      onMouseDown={() => getCurrentWindow().startDragging()}
      className="h-screen w-screen flex flex-col justify-end px-5 py-3 rounded-card select-none cursor-move"
      style={{ background: "rgba(12,12,14,.82)", backdropFilter: "blur(8px)" }}
    >
      {/* Dimmed source line */}
      <p className="text-white/50 text-[13px] truncate">{source}</p>
      {/* Target line — RTL Persian, partial at 50% opacity after final text */}
      <p lang="fa" className="fa-text text-white leading-snug" style={{ fontSize }}>
        {finalText}
        {partialText && <span className="opacity-50"> {partialText}</span>}
      </p>
      <footer className="flex justify-between text-[11px] text-white/40 mt-1">
        <span>{t("overlay.dragHint")}</span>
        <span>{behind.toFixed(1)}s behind live</span>
      </footer>
    </div>
  )
}

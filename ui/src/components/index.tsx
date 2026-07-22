// Shared components implementing the mockup's interaction principles:
// signal-path chips, VU meters, latency badges, segmented switches.
import React, { useEffect, useRef } from "react"

/** Live VU meter — "prove, don't promise": audio must be visibly flowing. */
export function VuMeter({ level, bars = 24 }: { level: number; bars?: number }) {
  const active = Math.round(Math.min(1, level * 3) * bars)
  return (
    <div className="flex items-end gap-[2px] h-6" role="meter" aria-label="Audio level"
         aria-valuenow={Math.round(level * 100)} aria-valuemin={0} aria-valuemax={100}>
      {Array.from({ length: bars }, (_, i) => (
        <div
          key={i}
          className={`w-[3px] rounded-sm transition-all duration-75 ${
            i < active ? (i > bars * 0.85 ? "bg-attention" : "bg-positive") : "bg-raised"
          }`}
          style={{ height: `${30 + (i / bars) * 70}%` }}
        />
      ))}
    </div>
  )
}

/** Signal-path chip — clickable, jumps to the relevant setting. */
export function Chip({ children, onClick, lang }: { children: React.ReactNode; onClick?: () => void; lang?: string }) {
  return (
    <button
      onClick={onClick}
      lang={lang}
      className="px-2 py-1 rounded bg-raised text-[12px] whitespace-nowrap hover:bg-white/20 focus:outline focus:outline-2 focus:outline-accent"
    >
      {children}
    </button>
  )
}

/** Chips-with-arrows signal path (audio routing always visible). */
export function ChipPath({ items }: { items: { label: string; onClick?: () => void; lang?: string }[] }) {
  return (
    <div className="flex items-center gap-1.5 flex-wrap">
      {items.map((item, i) => (
        <React.Fragment key={i}>
          {i > 0 && <span className="text-secondary text-[12px]">→</span>}
          <Chip onClick={item.onClick} lang={item.lang}>{item.label}</Chip>
        </React.Fragment>
      ))}
    </div>
  )
}

/** Small latency/status badge with honest numbers. */
export function Badge({ children, tone = "neutral" }: { children: React.ReactNode; tone?: "neutral" | "positive" | "attention" | "danger" }) {
  const tones = {
    neutral: "bg-raised text-secondary",
    positive: "bg-positive/20 text-positive",
    attention: "bg-attention/20 text-attention",
    danger: "bg-danger/20 text-danger",
  }
  return <span className={`px-1.5 py-0.5 rounded text-[12px] ${tones[tone]}`}>{children}</span>
}

/** Pipeline on/off toggle. */
export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative w-11 h-6 rounded-full transition-colors ${checked ? "bg-positive" : "bg-raised"}`}
    >
      <span className={`absolute top-0.5 w-5 h-5 rounded-full bg-white transition-all ${checked ? "start-[22px]" : "start-0.5"}`} />
    </button>
  )
}

/** Local ↔ API segmented switch — one anatomy for all three engine cards. */
export function Segmented({ options, value, onChange }: { options: string[]; value: string; onChange: (v: string) => void }) {
  return (
    <div className="inline-flex rounded bg-raised p-0.5" role="tablist">
      {options.map((opt) => (
        <button
          key={opt}
          role="tab"
          aria-selected={value === opt}
          onClick={() => onChange(opt)}
          className={`px-3 py-1 rounded text-[14px] ${value === opt ? "bg-accent text-white" : "text-secondary hover:text-white"}`}
        >
          {opt}
        </button>
      ))}
    </div>
  )
}

/** Inline mono mark driven by currentColor (brand/svg/simorgh-mark-mono.svg). */
export function MarkMono({ size = 24 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" fill="none" aria-hidden>
      <path d="M6 40 Q10 28 14 40 T22 40 T30 40 T38 40" stroke="currentColor" strokeWidth="4" strokeLinecap="round" fill="none" />
      <path d="M38 40 Q46 34 56 18 M38 40 Q48 38 58 28 M38 40 Q50 42 60 38" stroke="currentColor" strokeWidth="4" strokeLinecap="round" fill="none" />
    </svg>
  )
}

/** Poll a Tauri command on an interval (meters, latency, status). */
export function usePoll<T>(fn: () => Promise<T>, ms: number, onData: (data: T) => void) {
  const saved = useRef(onData)
  saved.current = onData
  useEffect(() => {
    let live = true
    const tick = async () => {
      try {
        const data = await fn()
        if (live) saved.current(data)
      } catch { /* backend not ready yet */ }
    }
    tick()
    const id = setInterval(tick, ms)
    return () => { live = false; clearInterval(id) }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ms])
}

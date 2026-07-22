// Typed wrappers around the Tauri IPC surface (crates/app/src/commands.rs).
import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export type PipelineKind = "subtitle" | "voice"

export interface DeviceInfo {
  id: string
  name: string
  kind: "input" | "output" | "loopback_source" | "virtual_mic_sink"
  is_default: boolean
  sample_rate: number | null
}

export interface EngineChoice {
  engine: string
  model: string | null
  base_url: string | null
  api_key_ref: string | null
  proxy: string | null
}

export interface AppConfig {
  general: {
    ui_language: string
    theme: string
    global_hotkey_toggle: string
    push_to_talk_hotkey: string
    open_mic: boolean
    telemetry: boolean
  }
  devices: {
    loopback_device: string | null
    mic_device: string | null
    virtual_mic_sink: string | null
  }
  engines: {
    stt_subtitle: EngineChoice
    stt_voice: EngineChoice
    translator_subtitle: EngineChoice
    translator_voice: EngineChoice
    tts: EngineChoice
    subtitle_pair: { source: string; target: string } | null
    voice_pair: { source: string; target: string } | null
  }
  subtitle: { font_size: number; opacity: number; click_through: boolean; x: number | null; y: number | null; width: number | null }
  policy: { silence_end_ms: number; max_chunk_ms: number; retranslate_debounce_ms: number }
  profiles: Record<string, unknown>
  active_profile: string | null
}

export interface TranscriptSegment {
  id: string
  text: string
  lang: string
  is_final: boolean
  t_start: number
  t_end: number
}

export interface TranslationUpdate {
  segment_id: string
  source_text: string
  translated_text: string
  lang: { source: string; target: string }
  is_final: boolean
}

export type PipelineEvent =
  | { type: "state"; kind: PipelineKind; state: string }
  | ({ type: "transcript" } & TranscriptSegment)
  | ({ type: "translation" } & TranslationUpdate)
  | { type: "level"; kind: PipelineKind; rms: number; peak: number }

export const getConfig = () => invoke<AppConfig>("get_config")
export const setConfig = (config: AppConfig) => invoke<void>("set_config", { config })
export const listDevices = () => invoke<DeviceInfo[]>("list_devices")
export const startPipeline = (kind: PipelineKind) => invoke<void>("start_pipeline", { kind })
export const stopPipeline = (kind: PipelineKind) => invoke<void>("stop_pipeline", { kind })
export const pipelineStatus = () => invoke<{ subtitle: string; voice: string }>("pipeline_status")
export const latencyReport = () => invoke<{ per_stage_ms: Record<string, number>; end_to_end_ms: number }>("latency_report")
export const meterLevels = () => invoke<{ subtitle: { rms: number; peak: number }; voice: { rms: number; peak: number } }>("meter_levels")
export const setApiKey = (entry: string, key: string) => invoke<void>("set_api_key", { entry, key })
export const ensureVirtualMic = () => invoke<{ status: string; device?: string; instructions?: string }>("ensure_virtual_mic")
export const listModels = () => invoke<{ name: string; kind: string; size_bytes: number | null; downloaded: boolean }[]>("list_models")
export const downloadModel = (name: string, url: string, sha256?: string) => invoke<void>("download_model", { name, url, sha256 })
export const deleteModel = (name: string) => invoke<void>("delete_model", { name })
export const exportHistory = (format: "txt" | "srt") => invoke<string>("export_history", { format })
export const setOverlayClickThrough = (enabled: boolean) => invoke<void>("set_overlay_click_through", { enabled })

export const onPipelineEvent = (cb: (ev: PipelineEvent) => void): Promise<UnlistenFn> =>
  listen<PipelineEvent>("pipeline-event", (e) => cb(e.payload))

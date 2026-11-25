export type PowerAction = "shutdown" | "restart" | "sleep" | "lock" | "logout"

export interface ClipboardItem {
  id: string
  content_type: "text" | "code" | "link" | "image"
  text_content?: string | null
  image_data_url?: string | null
  image_width?: number | null
  image_height?: number | null
  image_size_bytes?: number | null
  hash: string
  copied_at: string
  is_pinned: boolean
  char_count?: number | null
  language?: string | null
}

export interface ShutdownTimerStatus {
  active: boolean
  remaining_seconds: number
  action?: PowerAction
}

export interface ExplorerSettings {
  enabled: boolean
  include_extensions: boolean
}

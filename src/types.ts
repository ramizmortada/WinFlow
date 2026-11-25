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

export type ModuleId =
  | "clipboard"
  | "snipper"
  | "color_picker"
  | "black_screen"
  | "power_timer"
  | "explorer_copy_names"
  | "movie_posters"

export interface ModuleFlags {
  clipboard: boolean
  snipper: boolean
  color_picker: boolean
  black_screen: boolean
  power_timer: boolean
  explorer_copy_names: boolean
  movie_posters: boolean
}

export interface PosterCandidate {
  title: string
  year: string
  poster_url: string
  source: string
}

export interface SubfolderScanItem {
  path: string
  name: string
  parsed_query: string
  has_poster: boolean
}

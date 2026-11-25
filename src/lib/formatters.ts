export function formatTimeAMPM(timeStr?: string | null): string {
  if (!timeStr) return ""
  const trimmed = timeStr.trim()
  if (/am|pm/i.test(trimmed)) return trimmed

  const match = trimmed.match(/^(\d{1,2}):(\d{2})(?::(\d{2}))?$/)
  if (match) {
    let hours = parseInt(match[1], 10)
    const minutes = match[2]
    const ampm = hours >= 12 ? "PM" : "AM"
    hours = hours % 12 || 12
    return `${hours}:${minutes} ${ampm}`
  }

  const d = new Date(trimmed)
  if (!isNaN(d.getTime())) {
    return d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit", hour12: true })
  }

  return trimmed
}

export function formatKeyDisplayName(key: string): string {
  const k = key.trim()
  if (k.toLowerCase() === "super" || k.toLowerCase() === "meta") return "Win"
  if (k.toLowerCase() === "command") return "Cmd"
  if (k.toLowerCase() === "control") return "Ctrl"
  if (k.toLowerCase() === "backquote") return "`"
  return k
}

export function formatShortcutDisplay(shortcutStr: string): string {
  if (!shortcutStr) return ""
  return shortcutStr
    .split("+")
    .map((k) => formatKeyDisplayName(k))
    .join(" + ")
}

export function formatCountdown(totalSeconds: number): string {
  const hrs = Math.floor(totalSeconds / 3600)
  const mins = Math.floor((totalSeconds % 3600) / 60)
  const secs = totalSeconds % 60
  const pad = (n: number) => String(n).padStart(2, "0")
  return hrs > 0 ? `${pad(hrs)}:${pad(mins)}:${pad(secs)}` : `${pad(mins)}:${pad(secs)}`
}

export function isHexColorItem(item: { content_type: string; text_content?: string | null }): boolean {
  if (item.content_type === "image" || !item.text_content) return false
  const trimmed = item.text_content.trim()
  return /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(trimmed)
}

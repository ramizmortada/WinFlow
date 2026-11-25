import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import { Copy, Minus, ScanText, X } from "lucide-react"

export default function PinnedSnipWindow() {
  const [dataUrl, setDataUrl] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)

  useEffect(() => {
    invoke<string | null>("get_pinned_snip_data")
      .then((d) => {
        if (d) setDataUrl(d)
      })
      .catch(() => {})

    let unlisten: (() => void) | undefined
    listen<string>("pinned-snip-updated", (e) => {
      setDataUrl(e.payload)
      setNotice(null)
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(() => {})

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  const showToast = (msg: string) => {
    setNotice(msg)
    setTimeout(() => setNotice(null), 1400)
  }

  const handleStartDrag = (e: React.MouseEvent) => {
    if (e.button !== 0) return
    getCurrentWebviewWindow()
      .startDragging()
      .catch((err) => console.error("startDragging failed:", err))
  }

  const handleMinimize = (e: React.MouseEvent) => {
    e.stopPropagation()
    invoke("minimize_pinned_snip").catch(() => {
      getCurrentWebviewWindow()
        .minimize()
        .catch(() => {})
    })
  }

  return (
    <div className="relative w-screen h-screen overflow-hidden rounded-xl border border-emerald-600/60 bg-[#070f0a] shadow-2xl group select-none">
      {/* Draggable surface */}
      <div
        data-tauri-drag-region
        onMouseDown={handleStartDrag}
        className="w-full h-full flex items-center justify-center cursor-move"
      >
        {dataUrl && (
          <img
            src={dataUrl}
            alt="Pinned snip"
            draggable={false}
            className="w-full h-full object-contain pointer-events-none"
          />
        )}
      </div>

      {/* Hover controls */}
      <div
        onMouseDown={(e) => e.stopPropagation()}
        className="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity bg-[#07130c]/90 border border-emerald-700/60 rounded-lg p-1 backdrop-blur-md"
      >
        <button
          type="button"
          onClick={async () => {
            if (!dataUrl) return
            try {
              const text = await invoke<string>("ocr_snip_image", { dataUrl })
              showToast(text && text.trim() ? "Text copied!" : "No text found")
            } catch {
              showToast("OCR failed")
            }
          }}
          title="Copy Text (OCR)"
          className="p-1 rounded text-teal-300 hover:text-teal-200 hover:bg-teal-500/20"
        >
          <ScanText className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={() => {
            if (!dataUrl) return
            invoke("copy_snip_image", { dataUrl }).catch(() => {})
            showToast("Image copied!")
          }}
          title="Copy Image"
          className="p-1 rounded text-emerald-300 hover:text-[#14ad72] hover:bg-emerald-900/50"
        >
          <Copy className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={handleMinimize}
          title="Minimize to Taskbar"
          className="p-1 rounded text-emerald-300 hover:text-amber-300 hover:bg-emerald-900/50"
        >
          <Minus className="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          onClick={() => invoke("close_pinned_snip").catch(() => {})}
          title="Close Pinned Snip"
          className="p-1 rounded text-emerald-300 hover:text-red-300 hover:bg-red-500/20"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      {notice && (
        <div className="pointer-events-none absolute bottom-2 left-1/2 -translate-x-1/2 px-2.5 py-1 rounded-full bg-[#14ad72] text-slate-950 text-[10px] font-semibold shadow">
          {notice}
        </div>
      )}
    </div>
  )
}

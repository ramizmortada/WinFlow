import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { Copy, X } from "lucide-react"

export default function PinnedSnipWindow() {
  const [dataUrl, setDataUrl] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    invoke<string | null>("get_pinned_snip_data")
      .then((d) => {
        if (d) setDataUrl(d)
      })
      .catch(() => {})

    let unlisten: (() => void) | undefined
    listen<string>("pinned-snip-updated", (e) => {
      setDataUrl(e.payload)
      setCopied(false)
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(() => {})

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  return (
    <div className="relative w-screen h-screen overflow-hidden rounded-xl border border-emerald-600/60 bg-[#070f0a] shadow-2xl group select-none">
      {/* Draggable surface */}
      <div data-tauri-drag-region className="w-full h-full flex items-center justify-center cursor-move">
        {dataUrl && (
          <img
            src={dataUrl}
            alt="Pinned snip"
            data-tauri-drag-region
            className="w-full h-full object-contain pointer-events-none"
          />
        )}
      </div>

      {/* Hover controls */}
      <div className="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity bg-[#07130c]/90 border border-emerald-700/60 rounded-lg p-1 backdrop-blur-md">
        <button
          onClick={() => {
            if (!dataUrl) return
            invoke("copy_snip_image", { dataUrl }).catch(() => {})
            setCopied(true)
            setTimeout(() => setCopied(false), 1200)
          }}
          title="Copy to Clipboard"
          className="p-1 rounded text-emerald-300 hover:text-[#14ad72] hover:bg-emerald-900/50"
        >
          <Copy className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={() => invoke("close_pinned_snip").catch(() => {})}
          title="Close Pinned Snip"
          className="p-1 rounded text-emerald-300 hover:text-red-300 hover:bg-red-500/20"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>

      {copied && (
        <div className="absolute bottom-2 left-1/2 -translate-x-1/2 px-2.5 py-1 rounded-full bg-[#14ad72] text-slate-950 text-[10px] font-semibold">
          Copied!
        </div>
      )}
    </div>
  )
}

import type React from "react"
import { useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import {
  ArrowUpRight,
  Clock,
  Copy,
  Download,
  EyeOff,
  Highlighter,
  Maximize,
  MousePointer,
  Pin,
  RotateCcw,
  Scan,
  ScanText,
  Square,
  Type,
  Undo2,
  X,
  AppWindow,
} from "lucide-react"

interface WindowBounds {
  x: number
  y: number
  width: number
  height: number
  title: string
}

interface SnipSnapshotPayload {
  data_url: string
  offset_x: number
  offset_y: number
  width: number
  height: number
  windows: WindowBounds[]
}

interface Rect {
  x: number
  y: number
  w: number
  h: number
}

type CaptureMode = "region" | "window" | "fullscreen"
type AnnotTool = "none" | "arrow" | "rect" | "highlight" | "blur" | "text"

interface Annotation {
  tool: Exclude<AnnotTool, "none">
  color: string
  size: number
  x1: number
  y1: number
  x2: number
  y2: number
  points?: { x: number; y: number }[]
  text?: string
}

const COLORS = ["#ef4444", "#14ad72", "#f59e0b", "#3b82f6", "#ffffff"]

export default function SnipperOverlay() {
  const [snapshot, setSnapshot] = useState<SnipSnapshotPayload | null>(null)
  const [mode, setMode] = useState<CaptureMode>("region")
  const [delaySecs, setDelaySecs] = useState<number>(0)
  const [selection, setSelection] = useState<Rect | null>(null)
  const [hoverWindow, setHoverWindow] = useState<Rect | null>(null)
  const [isDragging, setIsDragging] = useState(false)
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null)

  // Annotations
  const [tool, setTool] = useState<AnnotTool>("none")
  const [color, setColor] = useState<string>("#ef4444")
  const [strokeSize, setStrokeSize] = useState<number>(3)
  const [annotations, setAnnotations] = useState<Annotation[]>([])
  const [activeAnnot, setActiveAnnot] = useState<Annotation | null>(null)
  const [textInput, setTextInput] = useState<{ x: number; y: number; value: string } | null>(null)
  const [busy, setBusy] = useState(false)

  const canvasRef = useRef<HTMLCanvasElement>(null)
  const imgRef = useRef<HTMLImageElement | null>(null)
  const [imgReady, setImgReady] = useState(0)
  const [viewportTick, setViewportTick] = useState(0)

  useEffect(() => {
    const onResize = () => setViewportTick((t) => t + 1)
    window.addEventListener("resize", onResize)
    return () => window.removeEventListener("resize", onResize)
  }, [])

  const resetOverlayState = () => {
    const canvas = canvasRef.current
    if (canvas) {
      const ctx = canvas.getContext("2d")
      if (ctx) {
        ctx.clearRect(0, 0, canvas.width, canvas.height)
      }
    }
    setSnapshot(null)
    imgRef.current = null
    setImgReady(0)
    setSelection(null)
    setHoverWindow(null)
    setAnnotations([])
    setActiveAnnot(null)
    setTextInput(null)
    setTool("none")
    setMode("region")
    setBusy(false)
  }

  const loadSnapshot = (payload: SnipSnapshotPayload) => {
    setSnapshot(payload)
    setSelection(null)
    setHoverWindow(null)
    setAnnotations([])
    setActiveAnnot(null)
    setTextInput(null)
    setTool("none")
    setBusy(false)

    const img = new Image()
    img.onload = () => {
      imgRef.current = img
      setImgReady((c) => c + 1)
    }
    img.src = payload.data_url
  }

  useEffect(() => {
    const fetchSnapshot = () => {
      resetOverlayState()
      invoke<SnipSnapshotPayload | null>("get_snip_snapshot")
        .then((p) => {
          if (p) loadSnapshot(p)
        })
        .catch(() => {})
    }

    fetchSnapshot()

    let unlisten: (() => void) | undefined
    listen("snipper-opened", () => {
      fetchSnapshot()
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(() => {})

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  // Redraw canvas whenever snapshot, selection, hoverWindow, or annotations change
  useEffect(() => {
    const canvas = canvasRef.current
    const img = imgRef.current
    if (!canvas) return
    if (!snapshot || !img) {
      const ctx = canvas.getContext("2d")
      if (ctx) ctx.clearRect(0, 0, canvas.width, canvas.height)
      return
    }

    const dpr = window.devicePixelRatio || 1
    const cssW = window.innerWidth
    const cssH = window.innerHeight
    canvas.width = cssW * dpr
    canvas.height = cssH * dpr

    const ctx = canvas.getContext("2d")
    if (!ctx) return
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

    // Draw frozen desktop image stretched 1:1 to viewport
    ctx.drawImage(img, 0, 0, cssW, cssH)

    // Dim overlay
    const activeRect = selection || (mode === "window" ? hoverWindow : null)
    ctx.fillStyle = "rgba(0, 0, 0, 0.48)"
    ctx.fillRect(0, 0, cssW, cssH)

    if (activeRect && activeRect.w > 2 && activeRect.h > 2) {
      // Clear dim inside active selection
      ctx.save()
      ctx.beginPath()
      ctx.rect(activeRect.x, activeRect.y, activeRect.w, activeRect.h)
      ctx.clip()
      ctx.drawImage(img, 0, 0, cssW, cssH)

      // Draw annotations clipped to selection
      const allAnnots = activeAnnot ? [...annotations, activeAnnot] : annotations
      drawAnnotations(ctx, img, cssW, cssH, allAnnots)
      ctx.restore()

      // Selection border
      ctx.strokeStyle = "#14ad72"
      ctx.lineWidth = 2
      ctx.strokeRect(activeRect.x, activeRect.y, activeRect.w, activeRect.h)

      // Dimension badge
      const label = `${Math.round(activeRect.w)} × ${Math.round(activeRect.h)}`
      ctx.font = "600 11px ui-monospace, SFMono-Regular, monospace"
      const tw = ctx.measureText(label).width
      const bx = Math.max(8, Math.min(cssW - tw - 20, activeRect.x))
      const by = activeRect.y > 28 ? activeRect.y - 24 : activeRect.y + 6
      ctx.fillStyle = "rgba(7, 15, 10, 0.9)"
      ctx.fillRect(bx, by, tw + 12, 20)
      ctx.fillStyle = "#14ad72"
      ctx.fillText(label, bx + 6, by + 14)
    }
  }, [snapshot, imgReady, viewportTick, selection, hoverWindow, mode, annotations, activeAnnot])

  const drawAnnotations = (
    ctx: CanvasRenderingContext2D,
    img: HTMLImageElement,
    cssW: number,
    cssH: number,
    list: Annotation[]
  ) => {
    for (const a of list) {
      ctx.save()
      if (a.tool === "rect") {
        ctx.strokeStyle = a.color
        ctx.lineWidth = a.size
        ctx.strokeRect(
          Math.min(a.x1, a.x2),
          Math.min(a.y1, a.y2),
          Math.abs(a.x2 - a.x1),
          Math.abs(a.y2 - a.y1)
        )
      } else if (a.tool === "highlight") {
        if (a.points && a.points.length > 0) {
          ctx.globalAlpha = 0.38
          ctx.strokeStyle = a.color
          ctx.lineWidth = Math.max(12, a.size * 4)
          ctx.lineCap = "round"
          ctx.lineJoin = "round"
          ctx.beginPath()
          ctx.moveTo(a.points[0].x, a.points[0].y)
          for (let i = 1; i < a.points.length; i++) {
            ctx.lineTo(a.points[i].x, a.points[i].y)
          }
          ctx.stroke()
        } else {
          ctx.fillStyle = a.color + "55"
          ctx.fillRect(
            Math.min(a.x1, a.x2),
            Math.min(a.y1, a.y2),
            Math.abs(a.x2 - a.x1),
            Math.abs(a.y2 - a.y1)
          )
        }
      } else if (a.tool === "blur") {
        const rx = Math.min(a.x1, a.x2)
        const ry = Math.min(a.y1, a.y2)
        const rw = Math.max(1, Math.abs(a.x2 - a.x1))
        const rh = Math.max(1, Math.abs(a.y2 - a.y1))
        // Pixelate effect using downscaled offscreen canvas
        const pxSize = Math.max(8, a.size * 3)
        const sw = Math.max(1, Math.floor(rw / pxSize))
        const sh = Math.max(1, Math.floor(rh / pxSize))
        const tmp = document.createElement("canvas")
        tmp.width = sw
        tmp.height = sh
        const tctx = tmp.getContext("2d")
        if (tctx) {
          const sx = (rx / cssW) * img.width
          const sy = (ry / cssH) * img.height
          const sWidth = (rw / cssW) * img.width
          const sHeight = (rh / cssH) * img.height
          tctx.drawImage(img, sx, sy, sWidth, sHeight, 0, 0, sw, sh)
          ctx.imageSmoothingEnabled = false
          ctx.drawImage(tmp, rx, ry, rw, rh)
        }
      } else if (a.tool === "arrow") {
        const headLen = 10 + a.size * 2
        const angle = Math.atan2(a.y2 - a.y1, a.x2 - a.x1)
        ctx.strokeStyle = a.color
        ctx.fillStyle = a.color
        ctx.lineWidth = a.size
        ctx.lineCap = "round"
        ctx.beginPath()
        ctx.moveTo(a.x1, a.y1)
        ctx.lineTo(a.x2, a.y2)
        ctx.stroke()

        ctx.beginPath()
        ctx.moveTo(a.x2, a.y2)
        ctx.lineTo(
          a.x2 - headLen * Math.cos(angle - Math.PI / 6),
          a.y2 - headLen * Math.sin(angle - Math.PI / 6)
        )
        ctx.lineTo(
          a.x2 - headLen * Math.cos(angle + Math.PI / 6),
          a.y2 - headLen * Math.sin(angle + Math.PI / 6)
        )
        ctx.closePath()
        ctx.fill()
      } else if (a.tool === "text" && a.text) {
        ctx.font = `600 ${13 + a.size * 2}px sans-serif`
        ctx.fillStyle = a.color
        ctx.fillText(a.text, a.x1, a.y1)
      }
      ctx.restore()
    }
  }

  const exportCroppedDataUrl = (): { dataUrl: string; width: number; height: number } | null => {
    const img = imgRef.current
    if (!img || !selection || selection.w < 2 || selection.h < 2) return null

    const cssW = window.innerWidth
    const cssH = window.innerHeight
    const scaleX = img.width / cssW
    const scaleY = img.height / cssH

    const outW = Math.max(1, Math.round(selection.w * scaleX))
    const outH = Math.max(1, Math.round(selection.h * scaleY))

    const out = document.createElement("canvas")
    out.width = outW
    out.height = outH
    const ctx = out.getContext("2d")
    if (!ctx) return null

    ctx.drawImage(
      img,
      selection.x * scaleX,
      selection.y * scaleY,
      outW,
      outH,
      0,
      0,
      outW,
      outH
    )

    // Draw annotations scaled into cropped space
    ctx.save()
    ctx.scale(scaleX, scaleY)
    ctx.translate(-selection.x, -selection.y)
    drawAnnotations(ctx, img, cssW, cssH, annotations)
    ctx.restore()

    return { dataUrl: out.toDataURL("image/png"), width: outW, height: outH }
  }

  const handleAction = async (kind: "copy" | "save" | "pin" | "ocr") => {
    if (busy) return
    const exported = exportCroppedDataUrl()
    if (!exported) return
    setBusy(true)
    try {
      if (kind === "copy") {
        await invoke("copy_snip_image", { dataUrl: exported.dataUrl })
      } else if (kind === "save") {
        await invoke("save_snip_image", { dataUrl: exported.dataUrl })
      } else if (kind === "pin") {
        await invoke("pin_snip_image", {
          dataUrl: exported.dataUrl,
          width: exported.width,
          height: exported.height,
        })
      } else if (kind === "ocr") {
        await invoke<string>("ocr_snip_image", { dataUrl: exported.dataUrl })
      }
      resetOverlayState()
    } catch (err) {
      console.error("Snipper action failed:", err)
    } finally {
      setBusy(false)
    }
  }

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (textInput) return
      if (e.key === "Escape") {
        resetOverlayState()
        invoke("close_snipper").catch(() => {})
      } else if (e.key === "Enter" || (e.ctrlKey && e.key.toLowerCase() === "c")) {
        if (selection) {
          e.preventDefault()
          handleAction("copy")
        }
      } else if (e.ctrlKey && e.key.toLowerCase() === "z") {
        setAnnotations((prev) => prev.slice(0, -1))
      }
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [selection, annotations, textInput, busy])

  const handleMouseDown = (e: React.MouseEvent) => {
    if (e.button !== 0) return
    const x = e.clientX
    const y = e.clientY

    if (selection && tool !== "none") {
      if (tool === "text") {
        setTextInput({ x, y, value: "" })
        return
      }
      setActiveAnnot({
        tool,
        color,
        size: strokeSize,
        x1: x,
        y1: y,
        x2: x,
        y2: y,
        points: tool === "highlight" ? [{ x, y }] : undefined,
      })
      return
    }

    if (mode === "window" && hoverWindow) {
      setSelection(hoverWindow)
      return
    }

    if (mode === "fullscreen") {
      setSelection({ x: 0, y: 0, w: window.innerWidth, h: window.innerHeight })
      return
    }

    setSelection(null)
    setAnnotations([])
    setIsDragging(true)
    setDragStart({ x, y })
  }

  const handleMouseMove = (e: React.MouseEvent) => {
    const x = e.clientX
    const y = e.clientY

    if (activeAnnot) {
      if (activeAnnot.tool === "highlight" && activeAnnot.points) {
        setActiveAnnot({
          ...activeAnnot,
          x2: x,
          y2: y,
          points: [...activeAnnot.points, { x, y }],
        })
      } else {
        setActiveAnnot({ ...activeAnnot, x2: x, y2: y })
      }
      return
    }

    if (isDragging && dragStart) {
      setSelection({
        x: Math.min(dragStart.x, x),
        y: Math.min(dragStart.y, y),
        w: Math.abs(x - dragStart.x),
        h: Math.abs(y - dragStart.y),
      })
      return
    }

    if (mode === "window" && !selection && snapshot) {
      const scaleX = window.innerWidth / snapshot.width
      const scaleY = window.innerHeight / snapshot.height
      const hit = snapshot.windows.find((w) => {
        const rx = w.x * scaleX
        const ry = w.y * scaleY
        const rw = w.width * scaleX
        const rh = w.height * scaleY
        return x >= rx && x <= rx + rw && y >= ry && y <= ry + rh
      })
      if (hit) {
        setHoverWindow({
          x: Math.max(0, hit.x * scaleX),
          y: Math.max(0, hit.y * scaleY),
          w: Math.min(window.innerWidth, hit.width * scaleX),
          h: Math.min(window.innerHeight, hit.height * scaleY),
        })
      } else {
        setHoverWindow(null)
      }
    }
  }

  const handleMouseUp = () => {
    if (activeAnnot) {
      setAnnotations((prev) => [...prev, activeAnnot])
      setActiveAnnot(null)
      return
    }
    if (isDragging) {
      setIsDragging(false)
      setDragStart(null)
    }
  }

  const triggerDelayedCapture = (secs: number) => {
    setDelaySecs(secs)
    invoke("delayed_snip_capture", { delaySeconds: secs }).catch(() => {})
  }

  // Position annotation bar cleanly below (or above/inside) selection without clipping
  const toolbarPos = selection
    ? {
        left: Math.max(12, Math.min(window.innerWidth - 580, selection.x)),
        top:
          selection.y + selection.h + 52 < window.innerHeight
            ? selection.y + selection.h + 8
            : selection.y > 106
            ? selection.y - 48
            : Math.min(window.innerHeight - 52, Math.max(58, selection.y + selection.h - 48)),
      }
    : null

  return (
    <div
      className={`fixed inset-0 w-screen h-screen overflow-hidden select-none ${
        imgReady === 0 ? "bg-black/50" : "bg-transparent"
      }`}
    >
      <canvas
        ref={canvasRef}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        className={`w-full h-full ${tool !== "none" && selection ? "cursor-crosshair" : "cursor-crosshair"}`}
      />

      {/* Floating Top Capture Bar */}
      <div
        onMouseDown={(e) => e.stopPropagation()}
        className="fixed top-3 left-1/2 -translate-x-1/2 z-50 flex items-center gap-1.5 px-2.5 py-1.5 rounded-2xl bg-[#07130c]/95 border border-emerald-700/60 shadow-2xl backdrop-blur-xl text-emerald-100"
      >
        {(
          [
            { id: "region", label: "Region", Icon: Scan },
            { id: "window", label: "Window", Icon: AppWindow },
            { id: "fullscreen", label: "Fullscreen", Icon: Maximize },
          ] as const
        ).map(({ id, label, Icon }) => (
          <button
            key={id}
            onClick={() => {
              setMode(id)
              if (id === "fullscreen") {
                setSelection({ x: 0, y: 0, w: window.innerWidth, h: window.innerHeight })
              } else {
                setSelection(null)
              }
            }}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-xs font-medium transition-colors ${
              mode === id
                ? "bg-[#14ad72] text-slate-950 font-semibold"
                : "text-emerald-200/80 hover:bg-emerald-900/50"
            }`}
          >
            <Icon className="h-3.5 w-3.5" />
            <span>{label}</span>
          </button>
        ))}

        <div className="h-4 w-[1px] bg-emerald-800/70 mx-1" />

        {/* Delay Timer Selector */}
        <div className="flex items-center gap-1 text-xs">
          <Clock className="h-3.5 w-3.5 text-emerald-400/70 mr-0.5" />
          {[0, 3, 5, 10].map((s) => (
            <button
              key={s}
              onClick={() => triggerDelayedCapture(s)}
              title={s === 0 ? "Capture immediately" : `Recapture after ${s}s delay`}
              className={`px-2 py-1 rounded-lg text-[11px] font-mono transition-colors ${
                delaySecs === s
                  ? "bg-emerald-500/20 text-[#14ad72] border border-[#14ad72]/50"
                  : "text-emerald-300/70 hover:bg-emerald-900/40"
              }`}
            >
              {s === 0 ? "0s" : `${s}s`}
            </button>
          ))}
        </div>

        <div className="h-4 w-[1px] bg-emerald-800/70 mx-1" />

        <button
          onClick={() => {
            resetOverlayState()
            invoke("close_snipper").catch(() => {})
          }}
          className="p-1.5 rounded-xl text-emerald-400 hover:text-red-300 hover:bg-red-500/15 transition-colors"
          title="Cancel (Esc)"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      {/* Inline Text Input when Text Tool clicked */}
      {textInput && (
        <input
          autoFocus
          value={textInput.value}
          onChange={(e) => setTextInput({ ...textInput, value: e.target.value })}
          onKeyDown={(e) => {
            if (e.key === "Enter" && textInput.value.trim()) {
              setAnnotations((prev) => [
                ...prev,
                {
                  tool: "text",
                  color,
                  size: strokeSize,
                  x1: textInput.x,
                  y1: textInput.y,
                  x2: textInput.x,
                  y2: textInput.y,
                  text: textInput.value,
                },
              ])
              setTextInput(null)
            } else if (e.key === "Escape") {
              setTextInput(null)
            }
          }}
          style={{ left: textInput.x, top: textInput.y - 16, color }}
          placeholder="Type & press Enter..."
          className="fixed z-50 px-2 py-1 rounded-md bg-black/80 border border-[#14ad72] text-sm font-semibold focus:outline-none"
        />
      )}

      {/* Floating Annotation & Action Bar */}
      {selection && !isDragging && toolbarPos && (
        <div
          onMouseDown={(e) => e.stopPropagation()}
          style={{ left: toolbarPos.left, top: toolbarPos.top }}
          className="fixed z-50 flex items-center gap-1 px-2 py-1.5 rounded-xl bg-[#07130c]/95 border border-emerald-700/70 shadow-2xl backdrop-blur-xl text-emerald-100"
        >
          {(
            [
              { id: "none", title: "Select", Icon: MousePointer },
              { id: "arrow", title: "Arrow", Icon: ArrowUpRight },
              { id: "rect", title: "Rectangle", Icon: Square },
              { id: "highlight", title: "Highlight", Icon: Highlighter },
              { id: "blur", title: "Blur / Redact", Icon: EyeOff },
              { id: "text", title: "Text", Icon: Type },
            ] as const
          ).map(({ id, title, Icon }) => (
            <button
              key={id}
              onClick={() => setTool(id)}
              title={title}
              className={`p-1.5 rounded-lg transition-colors ${
                tool === id
                  ? "bg-emerald-500/25 text-[#14ad72] border border-[#14ad72]/50"
                  : "text-emerald-300/80 hover:bg-emerald-900/50"
              }`}
            >
              <Icon className="h-3.5 w-3.5" />
            </button>
          ))}

          <div className="h-4 w-[1px] bg-emerald-800/70 mx-0.5" />

          {/* Colors */}
          <div className="flex items-center gap-1 px-1">
            {COLORS.map((c) => (
              <button
                key={c}
                onClick={() => setColor(c)}
                className={`h-3.5 w-3.5 rounded-full border transition-transform ${
                  color === c ? "scale-125 border-white" : "border-white/30"
                }`}
                style={{ backgroundColor: c }}
              />
            ))}
          </div>

          <button
            onClick={() => setStrokeSize((s) => (s === 2 ? 4 : s === 4 ? 7 : 2))}
            title="Cycle stroke size"
            className="px-1.5 py-1 rounded-md text-[10px] font-mono text-emerald-300 hover:bg-emerald-900/50"
          >
            {strokeSize}px
          </button>

          <button
            onClick={() => setAnnotations((prev) => prev.slice(0, -1))}
            title="Undo (Ctrl+Z)"
            className="p-1.5 rounded-lg text-emerald-300/80 hover:bg-emerald-900/50"
          >
            <Undo2 className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={() => setAnnotations([])}
            title="Clear annotations"
            className="p-1.5 rounded-lg text-emerald-300/80 hover:bg-emerald-900/50"
          >
            <RotateCcw className="h-3.5 w-3.5" />
          </button>

          <div className="h-4 w-[1px] bg-emerald-800/70 mx-0.5" />

          <button
            type="button"
            onClick={() => handleAction("ocr")}
            disabled={busy}
            title="Extract & Copy Text (OCR)"
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium text-teal-200 bg-teal-500/15 hover:bg-teal-500/25 border border-teal-500/30 transition-colors"
          >
            <ScanText className="h-3.5 w-3.5 text-teal-300" />
            <span>Copy Text</span>
          </button>
          <button
            type="button"
            onClick={() => handleAction("pin")}
            disabled={busy}
            title="Pin to Screen"
            className="flex items-center gap-1 px-2 py-1 rounded-lg text-xs text-amber-300 hover:bg-amber-500/15 transition-colors"
          >
            <Pin className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => handleAction("save")}
            disabled={busy}
            title="Save PNG"
            className="flex items-center gap-1 px-2 py-1 rounded-lg text-xs text-emerald-200 hover:bg-emerald-900/50 transition-colors"
          >
            <Download className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => handleAction("copy")}
            disabled={busy}
            title="Copy Image to Clipboard (Enter)"
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-semibold bg-[#14ad72] hover:bg-emerald-400 text-slate-950 transition-colors"
          >
            <Copy className="h-3.5 w-3.5" />
            <span>Copy Image</span>
          </button>
        </div>
      )}
    </div>
  )
}

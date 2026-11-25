import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow"
import {
  AlertCircle,
  Check,
  ClipboardList,
  Film,
  FolderOpen,
  Keyboard,
  Minus,
  Moon,
  Pipette,
  Play,
  Plus,
  Power,
  RefreshCw,
  RotateCcw,
  Scissors,
  Settings2,
  Sparkles,
  Trash2,
  X,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import PowerTimerPanel from "@/components/PowerTimerPanel"
import MoviePosterPanel from "@/components/MoviePosterPanel"
import { formatKeyDisplayName } from "@/lib/formatters"
import type { ExplorerSettings, ModuleFlags, ModuleId, ShutdownTimerStatus } from "@/types"

interface ModuleMeta {
  id: ModuleId
  name: string
  tagline: string
  description: string
  Icon: typeof ClipboardList
  launchable?: boolean
}

const MODULES: ModuleMeta[] = [
  {
    id: "clipboard",
    name: "Clipboard Manager",
    tagline: "History, pinning & instant paste",
    description:
      "Captures text, code, links, colors, and screenshots automatically. Press your shortcut near any input to paste.",
    Icon: ClipboardList,
    launchable: true,
  },
  {
    id: "snipper",
    name: "Snipping Tool & OCR",
    tagline: "Region, window, delay & Copy Text",
    description:
      "Freeze your screen to capture a region, window, or fullscreen with annotations, Pin to Screen, and Windows OCR.",
    Icon: Scissors,
    launchable: true,
  },
  {
    id: "color_picker",
    name: "Screen Color Picker",
    tagline: "Magnifying loupe & #HEX copy",
    description:
      "Sample any pixel on your desktop with a live magnifying grid and copy its #HEX code in one click.",
    Icon: Pipette,
    launchable: true,
  },
  {
    id: "black_screen",
    name: "Black Screen",
    tagline: "Pure black multi-monitor overlay",
    description:
      "Instantly blacks out all connected monitors with a hidden cursor. Press any key or click anywhere to exit.",
    Icon: Moon,
    launchable: true,
  },
  {
    id: "power_timer",
    name: "Power Timer",
    tagline: "Shutdown, restart, sleep, lock",
    description:
      "Schedule system power actions after a specific timespan (H:M:S) or at a specific time of day.",
    Icon: Power,
    launchable: false,
  },
  {
    id: "explorer_copy_names",
    name: "Explorer Copy Names",
    tagline: "Right-click file & folder names",
    description:
      "Adds Windows File Explorer right-click entries to copy all names inside a folder or from selected items.",
    Icon: FolderOpen,
    launchable: false,
  },
  {
    id: "movie_posters",
    name: "Movie Folder Posters",
    tagline: "Set folder icons to film posters",
    description:
      "Fetches movie posters via IMDb public search (no API key required) and sets them as Windows folder icons.",
    Icon: Film,
    launchable: false,
  },
]

export default function HubWindow() {
  const [selected, setSelected] = useState<ModuleId | "general">("clipboard")
  const [modules, setModules] = useState<ModuleFlags>({
    clipboard: true,
    snipper: true,
    color_picker: true,
    black_screen: true,
    power_timer: true,
    explorer_copy_names: true,
    movie_posters: false,
  })
  const [globalShortcuts, setGlobalShortcuts] = useState<string[]>(["Alt+V"])
  const [colorPickerShortcut, setColorPickerShortcut] = useState<string>("Alt+C")
  const [snipperShortcut, setSnipperShortcut] = useState<string>("Ctrl+Shift+S")
  const [explorerExt, setExplorerExt] = useState<boolean>(true)
  const [shutdownStatus, setShutdownStatus] = useState<ShutdownTimerStatus>({
    active: false,
    remaining_seconds: 0,
    action: "shutdown",
  })

  const [recordingTarget, setRecordingTarget] = useState<"clipboard" | "picker" | "snipper" | null>(
    null
  )
  const [recordedKeys, setRecordedKeys] = useState<string[]>([])
  const [feedback, setFeedback] = useState<{ type: "ok" | "err"; msg: string } | null>(null)

  useEffect(() => {
    invoke<ModuleFlags>("get_modules_config")
      .then((m) => setModules(m))
      .catch(() => {})
    invoke<string[]>("get_shortcuts")
      .then((s) => s && s.length > 0 && setGlobalShortcuts(s))
      .catch(() => {})
    invoke<string>("get_color_picker_shortcut")
      .then((s) => s && setColorPickerShortcut(s))
      .catch(() => {})
    invoke<string>("get_snipper_shortcut")
      .then((s) => s && setSnipperShortcut(s))
      .catch(() => {})
    invoke<ExplorerSettings>("get_explorer_settings")
      .then((e) => setExplorerExt(e.include_extensions))
      .catch(() => {})
    invoke<ShutdownTimerStatus>("get_shutdown_timer_status")
      .then((s) => setShutdownStatus(s))
      .catch(() => {})

    let unlistenMods: (() => void) | undefined
    listen<ModuleFlags>("modules-updated", (e) => setModules(e.payload))
      .then((fn) => {
        unlistenMods = fn
      })
      .catch(() => {})

    let unlistenTimer: (() => void) | undefined
    listen<ShutdownTimerStatus>("shutdown-timer-tick", (e) => setShutdownStatus(e.payload))
      .then((fn) => {
        unlistenTimer = fn
      })
      .catch(() => {})

    let unlistenSelect: (() => void) | undefined
    listen<ModuleId | "general">("hub-select-module", (e) => {
      setSelected(e.payload)
      setFeedback(null)
    })
      .then((fn) => {
        unlistenSelect = fn
      })
      .catch(() => {})

    return () => {
      if (unlistenMods) unlistenMods()
      if (unlistenTimer) unlistenTimer()
      if (unlistenSelect) unlistenSelect()
    }
  }, [])

  // Key recorder for shortcuts
  useEffect(() => {
    if (!recordingTarget) return

    const onKeyDown = async (e: KeyboardEvent) => {
      e.preventDefault()
      e.stopPropagation()

      if (e.key === "Escape") {
        setRecordingTarget(null)
        setRecordedKeys([])
        return
      }

      const mods: string[] = []
      if (e.ctrlKey) mods.push("Ctrl")
      if (e.altKey) mods.push("Alt")
      if (e.shiftKey) mods.push("Shift")
      if (e.metaKey) mods.push("Super")

      const rawKey = e.key
      if (["Control", "Alt", "Shift", "Meta", "OS"].includes(rawKey)) {
        setRecordedKeys(mods)
        return
      }

      let mainKey = rawKey.length === 1 ? rawKey.toUpperCase() : rawKey
      if (rawKey === " ") mainKey = "Space"
      if (rawKey === "`" || rawKey === "~") mainKey = "Backquote"

      const full = [...mods, mainKey]
      setRecordedKeys(full)

      if (mods.length > 0) {
        const candidate = full.join("+")
        setRecordingTarget(null)
        setRecordedKeys([])
        try {
          if (recordingTarget === "clipboard") {
            const updated = await invoke<string[]>("add_shortcut", { newShortcut: candidate })
            setGlobalShortcuts(updated)
          } else if (recordingTarget === "picker") {
            const updated = await invoke<string>("set_color_picker_shortcut", {
              newShortcut: candidate,
            })
            setColorPickerShortcut(updated)
          } else if (recordingTarget === "snipper") {
            const updated = await invoke<string>("set_snipper_shortcut", {
              newShortcut: candidate,
            })
            setSnipperShortcut(updated)
          }
          setFeedback({ type: "ok", msg: `Shortcut updated to ${candidate}` })
        } catch (err) {
          setFeedback({ type: "err", msg: String(err) })
        }
      } else {
        setFeedback({
          type: "err",
          msg: "Include at least one modifier key (Ctrl, Alt, Shift, or Win).",
        })
      }
    }

    window.addEventListener("keydown", onKeyDown, true)
    return () => window.removeEventListener("keydown", onKeyDown, true)
  }, [recordingTarget])

  const handleToggleModule = async (id: ModuleId) => {
    setFeedback(null)
    const next = !modules[id]
    try {
      const updated = await invoke<ModuleFlags>("set_module_enabled", {
        moduleId: id,
        enabled: next,
      })
      setModules(updated)
      setFeedback({
        type: "ok",
        msg: `${MODULES.find((m) => m.id === id)?.name} ${next ? "enabled" : "disabled"}.`,
      })
    } catch (err) {
      setFeedback({ type: "err", msg: String(err) })
    }
  }

  const handleLaunch = async (id: ModuleId) => {
    try {
      if (id === "clipboard") {
        await invoke("toggle_window")
      } else if (id === "snipper") {
        await invoke("start_snipper")
      } else if (id === "color_picker") {
        await invoke("start_color_picker")
      } else if (id === "black_screen") {
        await invoke("start_black_screen")
      } else if (id === "power_timer") {
        await invoke("open_tools_window")
      }
    } catch (err) {
      setFeedback({ type: "err", msg: String(err) })
    }
  }

  const currentMeta = MODULES.find((m) => m.id === selected)

  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden rounded-2xl border border-emerald-800/70 bg-[#070f0a] text-emerald-100 shadow-2xl select-none">
      {/* Draggable Titlebar */}
      <div
        onMouseDown={(e) => {
          if (e.button === 0) getCurrentWebviewWindow().startDragging().catch(() => {})
        }}
        className="flex items-center justify-between px-4 py-2.5 border-b border-emerald-900/60 bg-[#091710] cursor-move"
      >
        <div className="flex items-center gap-2">
          <div className="flex h-6 w-6 items-center justify-center rounded-lg bg-[#14ad72]/15 border border-[#14ad72]/30 text-[#14ad72]">
            <Sparkles className="h-3.5 w-3.5" />
          </div>
          <span className="text-xs font-semibold tracking-wide text-emerald-50">WinFlow Hub</span>
          <span className="px-1.5 py-0.5 rounded-md bg-emerald-950 border border-emerald-800/70 text-[10px] font-mono text-emerald-400">
            v0.2.6
          </span>
        </div>

        <div onMouseDown={(e) => e.stopPropagation()} className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => getCurrentWebviewWindow().minimize().catch(() => {})}
            className="p-1.5 rounded-lg text-emerald-400/70 hover:text-emerald-200 hover:bg-emerald-900/40"
            title="Minimize"
          >
            <Minus className="h-3.5 w-3.5" />
          </button>
          <button
            type="button"
            onClick={() => invoke("close_hub_window").catch(() => getCurrentWebviewWindow().hide().catch(() => {}))}
            className="p-1.5 rounded-lg text-emerald-400/70 hover:text-red-300 hover:bg-red-500/15"
            title="Close to Tray"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      {/* Two-Column Body */}
      <div className="flex flex-1 min-h-0">
        {/* Left Sidebar */}
        <div className="w-[225px] shrink-0 border-r border-emerald-900/60 bg-[#060d09] p-2.5 flex flex-col justify-between">
          <div className="space-y-1">
            <span className="px-2 text-[10px] font-semibold uppercase tracking-wider text-emerald-500/70 block mb-1.5">
              Modules
            </span>
            {MODULES.map((m) => {
              const active = selected === m.id
              const enabled = modules[m.id]
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => {
                    setSelected(m.id)
                    setFeedback(null)
                    setRecordingTarget(null)
                  }}
                  className={`w-full flex items-center justify-between px-2.5 py-2 rounded-xl text-left transition-colors ${
                    active
                      ? "bg-emerald-500/15 border border-[#14ad72]/40 text-emerald-50"
                      : "border border-transparent text-emerald-200/75 hover:bg-emerald-900/30"
                  }`}
                >
                  <span className="flex items-center gap-2 min-w-0">
                    <m.Icon
                      className={`h-3.5 w-3.5 shrink-0 ${
                        enabled ? "text-[#14ad72]" : "text-emerald-700"
                      }`}
                    />
                    <span className="text-xs font-medium truncate">{m.name}</span>
                  </span>
                  <span
                    className={`h-2 w-2 rounded-full shrink-0 ${
                      enabled ? "bg-[#14ad72]" : "bg-emerald-900"
                    }`}
                    title={enabled ? "Enabled" : "Disabled"}
                  />
                </button>
              )
            })}
          </div>

          <div className="pt-2 border-t border-emerald-900/50">
            <button
              type="button"
              onClick={() => {
                setSelected("general")
                setFeedback(null)
                setRecordingTarget(null)
              }}
              className={`w-full flex items-center gap-2 px-2.5 py-2 rounded-xl text-left transition-colors ${
                selected === "general"
                  ? "bg-emerald-500/15 border border-[#14ad72]/40 text-emerald-50"
                  : "border border-transparent text-emerald-200/75 hover:bg-emerald-900/30"
              }`}
            >
              <Settings2 className="h-3.5 w-3.5 text-[#14ad72]" />
              <span className="text-xs font-medium">General &amp; Updates</span>
            </button>
          </div>
        </div>

        {/* Right Detail Pane */}
        <div className="flex-1 min-w-0 p-5 overflow-y-auto flex flex-col justify-between">
          {selected === "general" ? (
            <div className="space-y-4">
              <div>
                <h2 className="text-base font-semibold text-emerald-50">General &amp; Updates</h2>
                <p className="text-xs text-emerald-400/70 mt-0.5">
                  Manage global WinFlow preferences, shortcuts, and application updates.
                </p>
              </div>

              <div className="p-3.5 rounded-xl bg-[#0b1811] border border-emerald-900/70 space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold text-emerald-100">Reset All Shortcuts</p>
                    <p className="text-[11px] text-emerald-400/60">
                      Restores Alt+V, Alt+C, and Ctrl+Shift+S
                    </p>
                  </div>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={async () => {
                      try {
                        const c = await invoke<string[]>("reset_shortcuts")
                        const p = await invoke<string>("set_color_picker_shortcut", {
                          newShortcut: "Alt+C",
                        })
                        const s = await invoke<string>("set_snipper_shortcut", {
                          newShortcut: "Ctrl+Shift+S",
                        })
                        setGlobalShortcuts(c)
                        setColorPickerShortcut(p)
                        setSnipperShortcut(s)
                        setFeedback({ type: "ok", msg: "All shortcuts reset to defaults." })
                      } catch (err) {
                        setFeedback({ type: "err", msg: String(err) })
                      }
                    }}
                    className="h-7 px-2.5 text-xs border-emerald-800 hover:bg-emerald-900/40 text-emerald-200"
                  >
                    <RotateCcw className="h-3 w-3 mr-1" />
                    Reset
                  </Button>
                </div>

                <div className="pt-2.5 border-t border-emerald-900/50 flex items-center justify-between">
                  <div>
                    <p className="text-xs font-semibold text-emerald-100">Check for Updates</p>
                    <p className="text-[11px] text-emerald-400/60">
                      Opens WinFlow updater to check GitHub Releases
                    </p>
                  </div>
                  <Button
                    size="sm"
                    onClick={() => {
                      invoke("toggle_window").catch(() => {})
                    }}
                    className="h-7 px-2.5 text-xs bg-[#14ad72] hover:bg-emerald-400 text-slate-950 font-semibold"
                  >
                    <RefreshCw className="h-3 w-3 mr-1" />
                    Open Clipboard
                  </Button>
                </div>
              </div>
            </div>
          ) : (
            currentMeta && (
              <div className="space-y-4">
                {/* Header + Enable Switch */}
                <div className="flex items-start justify-between gap-4 pb-3 border-b border-emerald-900/60">
                  <div className="flex items-start gap-3">
                    <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-emerald-500/10 border border-emerald-500/25 text-[#14ad72] shrink-0">
                      <currentMeta.Icon className="h-4.5 w-4.5" />
                    </div>
                    <div>
                      <h2 className="text-sm font-semibold text-emerald-50">{currentMeta.name}</h2>
                      <p className="text-[11px] text-emerald-400/70 mt-0.5 leading-relaxed">
                        {currentMeta.description}
                      </p>
                    </div>
                  </div>

                  <button
                    type="button"
                    onClick={() => handleToggleModule(currentMeta.id)}
                    className={`inline-flex h-5 w-9 shrink-0 items-center rounded-full p-0.5 transition-colors ${
                      modules[currentMeta.id]
                        ? "bg-[#14ad72]"
                        : "bg-[#06100a] border border-emerald-800"
                    }`}
                    title={modules[currentMeta.id] ? "Disable Module" : "Enable Module"}
                  >
                    <span
                      className={`h-4 w-4 rounded-full transition-transform ${
                        modules[currentMeta.id]
                          ? "translate-x-4 bg-slate-950"
                          : "translate-x-0 bg-emerald-400/60"
                      }`}
                    />
                  </button>
                </div>

                {/* Quick Launch */}
                {currentMeta.launchable && (
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[#0b1811] border border-emerald-900/70">
                    <div>
                      <p className="text-xs font-medium text-emerald-100">Quick Launch</p>
                      <p className="text-[10.5px] text-emerald-400/60">
                        Open {currentMeta.name} immediately
                      </p>
                    </div>
                    <Button
                      size="sm"
                      disabled={!modules[currentMeta.id]}
                      onClick={() => handleLaunch(currentMeta.id)}
                      className="h-7 px-3 text-xs bg-[#14ad72] hover:bg-emerald-400 text-slate-950 font-semibold"
                    >
                      <Play className="h-3 w-3 mr-1" />
                      Launch
                    </Button>
                  </div>
                )}

                {/* Shortcut Section for Clipboard / Snipper / Color Picker */}
                {(currentMeta.id === "clipboard" ||
                  currentMeta.id === "snipper" ||
                  currentMeta.id === "color_picker") && (
                  <div className="p-3 rounded-xl bg-[#0b1811] border border-emerald-900/70 space-y-2">
                    <div className="flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-emerald-400/80">
                      <Keyboard className="h-3 w-3" />
                      <span>Global Shortcut</span>
                    </div>

                    {currentMeta.id === "clipboard" ? (
                      <div className="space-y-1.5">
                        {globalShortcuts.map((sc) => (
                          <div
                            key={sc}
                            className="flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-[#06100a] border border-emerald-900/70"
                          >
                            <div className="flex items-center gap-1">
                              {sc.split("+").map((k, idx) => (
                                <kbd
                                  key={idx}
                                  className="rounded bg-emerald-950 px-1.5 py-0.5 border border-emerald-700/60 font-mono text-[10px] font-semibold text-[#14ad72]"
                                >
                                  {formatKeyDisplayName(k)}
                                </kbd>
                              ))}
                            </div>
                            {globalShortcuts.length > 1 && (
                              <button
                                type="button"
                                onClick={async () => {
                                  const updated = await invoke<string[]>("remove_shortcut", {
                                    shortcutToRemove: sc,
                                  })
                                  setGlobalShortcuts(updated)
                                }}
                                className="p-1 text-emerald-500/60 hover:text-red-400"
                              >
                                <Trash2 className="h-3 w-3" />
                              </button>
                            )}
                          </div>
                        ))}
                        <button
                          type="button"
                          onClick={() => {
                            setRecordingTarget("clipboard")
                            setRecordedKeys([])
                          }}
                          className="w-full py-1.5 rounded-lg border border-dashed border-emerald-800/80 hover:border-[#14ad72]/60 text-[11px] text-emerald-300 flex items-center justify-center gap-1"
                        >
                          <Plus className="h-3 w-3 text-[#14ad72]" />
                          <span>
                            {recordingTarget === "clipboard"
                              ? recordedKeys.length > 0
                                ? recordedKeys.join(" + ")
                                : "Press keys (Esc to cancel)..."
                              : "Add Shortcut"}
                          </span>
                        </button>
                      </div>
                    ) : (
                      <div className="flex items-center justify-between px-2.5 py-1.5 rounded-lg bg-[#06100a] border border-emerald-900/70">
                        <div className="flex items-center gap-1">
                          {(currentMeta.id === "snipper"
                            ? snipperShortcut
                            : colorPickerShortcut
                          )
                            .split("+")
                            .map((k, idx) => (
                              <kbd
                                key={idx}
                                className="rounded bg-emerald-950 px-1.5 py-0.5 border border-emerald-700/60 font-mono text-[10px] font-semibold text-[#14ad72]"
                              >
                                {formatKeyDisplayName(k)}
                              </kbd>
                            ))}
                        </div>
                        <button
                          type="button"
                          onClick={() => {
                            setRecordingTarget(
                              currentMeta.id === "snipper" ? "snipper" : "picker"
                            )
                            setRecordedKeys([])
                          }}
                          className="text-[11px] text-emerald-400 hover:text-[#14ad72] underline underline-offset-2"
                        >
                          {recordingTarget === "snipper" || recordingTarget === "picker"
                            ? recordedKeys.length > 0
                              ? recordedKeys.join(" + ")
                              : "Press keys..."
                            : "Change"}
                        </button>
                      </div>
                    )}
                  </div>
                )}

                {/* Explorer Copy Names Options */}
                {currentMeta.id === "explorer_copy_names" && (
                  <div className="p-3 rounded-xl bg-[#0b1811] border border-emerald-900/70">
                    <button
                      type="button"
                      onClick={async () => {
                        const res = await invoke<ExplorerSettings>(
                          "set_explorer_include_extensions",
                          { includeExtensions: !explorerExt }
                        )
                        setExplorerExt(res.include_extensions)
                        setFeedback({
                          type: "ok",
                          msg: res.include_extensions
                            ? "Copying names with file extensions."
                            : "Copying names without file extensions.",
                        })
                      }}
                      className="w-full flex items-center justify-between text-left"
                    >
                      <div>
                        <p className="text-xs font-medium text-emerald-100">
                          Include File Extensions
                        </p>
                        <p className="text-[10.5px] text-emerald-400/60">
                          {explorerExt ? "e.g. report.pdf, photo.png" : "e.g. report, photo"}
                        </p>
                      </div>
                      <span
                        className={`inline-flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors ${
                          explorerExt ? "bg-[#14ad72]" : "bg-[#06100a] border border-emerald-800"
                        }`}
                      >
                        <span
                          className={`h-3 w-3 rounded-full transition-transform ${
                            explorerExt
                              ? "translate-x-3 bg-slate-950"
                              : "translate-x-0 bg-emerald-400/60"
                          }`}
                        />
                      </span>
                    </button>
                  </div>
                )}

                {/* Embedded Power Timer Controls */}
                {currentMeta.id === "power_timer" && (
                  <PowerTimerPanel
                    shutdownStatus={shutdownStatus}
                    disabled={!modules.power_timer}
                    onShutdownStatusChange={setShutdownStatus}
                  />
                )}

                {/* Embedded Movie Folder Posters Panel */}
                {currentMeta.id === "movie_posters" && (
                  <MoviePosterPanel disabled={!modules.movie_posters} />
                )}
              </div>
            )
          )}

          {/* Feedback banner */}
          {feedback && (
            <div
              className={`mt-3 flex items-center gap-1.5 p-2 rounded-lg text-[11px] border ${
                feedback.type === "ok"
                  ? "bg-emerald-950/80 border-[#14ad72]/50 text-emerald-200"
                  : "bg-red-950/80 border-red-500/40 text-red-200"
              }`}
            >
              {feedback.type === "ok" ? (
                <Check className="h-3.5 w-3.5 text-[#14ad72] shrink-0" />
              ) : (
                <AlertCircle className="h-3.5 w-3.5 text-red-400 shrink-0" />
              )}
              <span>{feedback.msg}</span>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

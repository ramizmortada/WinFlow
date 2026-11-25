import { invoke } from "@tauri-apps/api/core"
import {
  AlertCircle,
  Check,
  FolderOpen,
  Keyboard,
  Pipette,
  Plus,
  RotateCcw,
  Scissors,
  Trash2,
  X,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { formatKeyDisplayName } from "@/lib/formatters"
import type { ExplorerSettings } from "@/types"

interface SettingsModalProps {
  globalShortcuts: string[]
  colorPickerShortcut: string
  snipperShortcut: string
  recordingTarget: "clipboard" | "picker" | "snipper" | null
  recordedKeys: string[]
  shortcutError: string | null
  shortcutSuccess: string | null
  explorerMenuEnabled: boolean
  explorerIncludeExt: boolean
  onClose: () => void
  onStartRecording: (target: "clipboard" | "picker" | "snipper") => void
  onRemoveShortcut: (sc: string) => void
  onResetDefaults: () => void
  onExplorerMenuChange: (enabled: boolean) => void
  onExplorerExtChange: (includeExt: boolean) => void
  onFeedback: (success: string | null, error: string | null) => void
}

export default function SettingsModal({
  globalShortcuts,
  colorPickerShortcut,
  snipperShortcut,
  recordingTarget,
  recordedKeys,
  shortcutError,
  shortcutSuccess,
  explorerMenuEnabled,
  explorerIncludeExt,
  onClose,
  onStartRecording,
  onRemoveShortcut,
  onResetDefaults,
  onExplorerMenuChange,
  onExplorerExtChange,
  onFeedback,
}: SettingsModalProps) {
  return (
    <div
      onClick={onClose}
      className="absolute inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md p-3.5 animate-in fade-in-50"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="flex flex-col p-4 rounded-2xl bg-[#091811] border border-emerald-800/80 shadow-2xl max-w-[320px] w-full text-emerald-100 animate-in zoom-in-95"
      >
        {/* Modal Header */}
        <div className="flex items-center justify-between pb-2.5 border-b border-emerald-900/60">
          <div className="flex items-center gap-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-emerald-500/10 text-[#14ad72] border border-emerald-500/20">
              <Keyboard className="h-4 w-4" />
            </div>
            <div>
              <h4 className="text-xs font-semibold text-emerald-100">WinFlow Settings</h4>
              <p className="text-[10px] text-emerald-400/60">Shortcuts &amp; Explorer tools</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-md text-emerald-400 hover:text-emerald-200 hover:bg-emerald-900/40"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Modal Body */}
        <div className="py-2.5 space-y-3.5 max-h-[300px] overflow-y-auto pr-0.5">
          {/* Section 1: Clipboard Palette Shortcuts */}
          <div className="space-y-1.5">
            <span className="text-[10px] font-medium text-emerald-400/70 uppercase tracking-wider block">
              Toggle Clipboard Manager:
            </span>

            <div className="space-y-1">
              {globalShortcuts.map((sc, i) => (
                <div
                  key={sc + i}
                  className="flex items-center justify-between px-2 py-1 rounded-lg bg-[#06100a] border border-emerald-900/70"
                >
                  <div className="flex items-center gap-1">
                    {sc.split("+").map((key, kIdx) => (
                      <kbd
                        key={kIdx}
                        className="rounded-md bg-emerald-950 px-1.5 py-0.5 border border-emerald-700/60 font-mono text-[10px] font-semibold text-[#14ad72]"
                      >
                        {formatKeyDisplayName(key)}
                      </kbd>
                    ))}
                  </div>

                  {globalShortcuts.length > 1 && (
                    <button
                      onClick={() => onRemoveShortcut(sc)}
                      className="p-1 rounded text-emerald-500/60 hover:text-red-400 hover:bg-red-500/10 transition-colors"
                      title="Remove this shortcut"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  )}
                </div>
              ))}
            </div>

            {/* Add new clipboard shortcut button */}
            <div
              onClick={() => onStartRecording("clipboard")}
              className={`relative flex flex-col items-center justify-center p-2 rounded-lg border text-center transition-all cursor-pointer ${
                recordingTarget === "clipboard"
                  ? "border-[#14ad72] bg-emerald-950/90 ring-2 ring-[#14ad72]/40 animate-pulse"
                  : "border-dashed border-emerald-800/80 bg-[#06100a]/50 hover:border-[#14ad72]/60 hover:bg-[#0c1d14]"
              }`}
            >
              {recordingTarget === "clipboard" ? (
                <div className="space-y-0.5">
                  <p className="text-[11px] font-semibold text-[#14ad72]">
                    {recordedKeys.length > 0
                      ? recordedKeys.map((k) => formatKeyDisplayName(k)).join(" + ")
                      : "Press keys to add (e.g. Win+V)..."}
                  </p>
                  <p className="text-[9px] text-emerald-400/60">Press Esc to cancel</p>
                </div>
              ) : (
                <div className="flex items-center gap-1 text-emerald-300 hover:text-[#14ad72]">
                  <Plus className="h-3 w-3 text-[#14ad72]" />
                  <span className="text-[10.5px] font-medium">Add Another Shortcut</span>
                </div>
              )}
            </div>
          </div>

          {/* Section 2: Screen Color Picker Shortcut */}
          <div className="space-y-1.5 pt-2 border-t border-emerald-900/60">
            <span className="text-[10px] font-medium text-emerald-400/70 uppercase tracking-wider block">
              Screen Color Picker:
            </span>

            <div className="flex items-center justify-between px-2 py-1 rounded-lg bg-[#06100a] border border-emerald-900/70">
              <div className="flex items-center gap-1.5">
                <Pipette className="h-3.5 w-3.5 text-[#14ad72]" />
                <div className="flex items-center gap-1">
                  {colorPickerShortcut.split("+").map((key, kIdx) => (
                    <kbd
                      key={kIdx}
                      className="rounded-md bg-emerald-950 px-1.5 py-0.5 border border-emerald-700/60 font-mono text-[10px] font-semibold text-[#14ad72]"
                    >
                      {formatKeyDisplayName(key)}
                    </kbd>
                  ))}
                </div>
              </div>

              <button
                onClick={() => onStartRecording("picker")}
                className="text-[10px] text-emerald-400 hover:text-[#14ad72] underline underline-offset-2"
              >
                Change
              </button>
            </div>

            {recordingTarget === "picker" && (
              <div className="relative flex flex-col items-center justify-center p-2 rounded-lg border border-[#14ad72] bg-emerald-950/90 ring-2 ring-[#14ad72]/40 animate-pulse text-center">
                <p className="text-[11px] font-semibold text-[#14ad72]">
                  {recordedKeys.length > 0
                    ? recordedKeys.map((k) => formatKeyDisplayName(k)).join(" + ")
                    : "Press new Color Picker keys (e.g. Alt+C)..."}
                </p>
                <p className="text-[9px] text-emerald-400/60">Press Esc to cancel</p>
              </div>
            )}
          </div>

          {/* Section 2b: Snipping Tool Shortcut */}
          <div className="space-y-1.5 pt-2 border-t border-emerald-900/60">
            <span className="text-[10px] font-medium text-emerald-400/70 uppercase tracking-wider block">
              Snipping Tool:
            </span>

            <div className="flex items-center justify-between px-2 py-1 rounded-lg bg-[#06100a] border border-emerald-900/70">
              <div className="flex items-center gap-1.5">
                <Scissors className="h-3.5 w-3.5 text-[#14ad72]" />
                <div className="flex items-center gap-1">
                  {snipperShortcut.split("+").map((key, kIdx) => (
                    <kbd
                      key={kIdx}
                      className="rounded-md bg-emerald-950 px-1.5 py-0.5 border border-emerald-700/60 font-mono text-[10px] font-semibold text-[#14ad72]"
                    >
                      {formatKeyDisplayName(key)}
                    </kbd>
                  ))}
                </div>
              </div>

              <button
                onClick={() => onStartRecording("snipper")}
                className="text-[10px] text-emerald-400 hover:text-[#14ad72] underline underline-offset-2"
              >
                Change
              </button>
            </div>

            {recordingTarget === "snipper" && (
              <div className="relative flex flex-col items-center justify-center p-2 rounded-lg border border-[#14ad72] bg-emerald-950/90 ring-2 ring-[#14ad72]/40 animate-pulse text-center">
                <p className="text-[11px] font-semibold text-[#14ad72]">
                  {recordedKeys.length > 0
                    ? recordedKeys.map((k) => formatKeyDisplayName(k)).join(" + ")
                    : "Press new Snipping Tool keys (e.g. Alt+S)..."}
                </p>
                <p className="text-[9px] text-emerald-400/60">Press Esc to cancel</p>
              </div>
            )}
          </div>

          {/* Section 3: File Explorer Context Menu */}
          <div className="space-y-2 pt-2 border-t border-emerald-900/60">
            <span className="text-[10px] font-medium text-emerald-400/70 uppercase tracking-wider block">
              File Explorer Right-Click Menu:
            </span>

            <div className="space-y-1.5">
              <button
                type="button"
                onClick={async () => {
                  const next = !explorerMenuEnabled
                  onFeedback(null, null)
                  try {
                    const res = await invoke<ExplorerSettings>("set_explorer_context_menu", {
                      enabled: next,
                    })
                    onExplorerMenuChange(res.enabled)
                    onFeedback(
                      res.enabled
                        ? "Explorer right-click menu enabled."
                        : "Explorer right-click menu removed.",
                      null
                    )
                  } catch (err) {
                    onFeedback(null, String(err))
                  }
                }}
                className="w-full flex items-center justify-between px-2.5 py-2 rounded-lg bg-[#06100a] border border-emerald-900/70 hover:border-emerald-700/70 transition-colors text-left"
              >
                <div className="flex items-center gap-2 pr-2">
                  <FolderOpen className="h-3.5 w-3.5 text-[#14ad72] shrink-0" />
                  <div>
                    <p className="text-[11px] font-medium text-emerald-100">
                      Copy File &amp; Folder Names
                    </p>
                    <p className="text-[9px] text-emerald-400/60">
                      Adds right-click option in File Explorer
                    </p>
                  </div>
                </div>
                <span
                  className={`inline-flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors ${
                    explorerMenuEnabled ? "bg-[#14ad72]" : "bg-emerald-950 border border-emerald-800"
                  }`}
                >
                  <span
                    className={`h-3 w-3 rounded-full bg-slate-950 transition-transform ${
                      explorerMenuEnabled ? "translate-x-3" : "translate-x-0 bg-emerald-400/60"
                    }`}
                  />
                </span>
              </button>

              <button
                type="button"
                onClick={async () => {
                  const next = !explorerIncludeExt
                  onFeedback(null, null)
                  try {
                    const res = await invoke<ExplorerSettings>("set_explorer_include_extensions", {
                      includeExtensions: next,
                    })
                    onExplorerExtChange(res.include_extensions)
                    onFeedback(
                      res.include_extensions
                        ? "Copying names with file extensions."
                        : "Copying names without file extensions.",
                      null
                    )
                  } catch (err) {
                    onFeedback(null, String(err))
                  }
                }}
                className="w-full flex items-center justify-between px-2.5 py-2 rounded-lg bg-[#06100a] border border-emerald-900/70 hover:border-emerald-700/70 transition-colors text-left"
              >
                <div className="pr-2">
                  <p className="text-[11px] font-medium text-emerald-100">
                    Include File Extensions
                  </p>
                  <p className="text-[9px] text-emerald-400/60">
                    {explorerIncludeExt ? "e.g. report.pdf, photo.png" : "e.g. report, photo"}
                  </p>
                </div>
                <span
                  className={`inline-flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors ${
                    explorerIncludeExt ? "bg-[#14ad72]" : "bg-emerald-950 border border-emerald-800"
                  }`}
                >
                  <span
                    className={`h-3 w-3 rounded-full bg-slate-950 transition-transform ${
                      explorerIncludeExt ? "translate-x-3" : "translate-x-0 bg-emerald-400/60"
                    }`}
                  />
                </span>
              </button>
            </div>
          </div>

          {/* Status feedback message */}
          {shortcutSuccess && (
            <div className="flex items-center gap-1.5 mt-2 p-1.5 rounded-lg bg-emerald-950/90 border border-[#14ad72]/60 text-[10px] text-emerald-200">
              <Check className="h-3 w-3 text-[#14ad72] shrink-0" />
              <span>{shortcutSuccess}</span>
            </div>
          )}

          {shortcutError && (
            <div className="flex items-center gap-1.5 mt-2 p-1.5 rounded-lg bg-red-950/80 border border-red-500/40 text-[10px] text-red-200">
              <AlertCircle className="h-3 w-3 text-red-400 shrink-0" />
              <span>{shortcutError}</span>
            </div>
          )}
        </div>

        {/* Modal Footer */}
        <div className="flex items-center justify-between pt-2 border-t border-emerald-900/60 mt-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={onResetDefaults}
            className="h-7 px-2 text-[10px] text-emerald-400/60 hover:text-emerald-200"
          >
            <RotateCcw className="h-3 w-3 mr-1" />
            Reset Defaults
          </Button>
          <Button
            size="sm"
            onClick={onClose}
            className="h-7 px-3 text-[11px] bg-[#14ad72] hover:bg-emerald-400 text-slate-950 font-semibold"
          >
            Done
          </Button>
        </div>
      </div>
    </div>
  )
}

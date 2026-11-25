import { useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import * as DropdownMenu from "@radix-ui/react-dropdown-menu"
import {
  AlertCircle,
  Check,
  ChevronDown,
  ChevronUp,
  Lock,
  LogOut,
  Moon,
  Power,
  RotateCcw,
  Timer,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { formatCountdown } from "@/lib/formatters"
import type { PowerAction, ShutdownTimerStatus } from "@/types"

interface PowerTimerPanelProps {
  shutdownStatus: ShutdownTimerStatus
  disabled?: boolean
  onShutdownStatusChange: (status: ShutdownTimerStatus) => void
}

export default function PowerTimerPanel({
  shutdownStatus,
  disabled = false,
  onShutdownStatusChange,
}: PowerTimerPanelProps) {
  const [powerAction, setPowerAction] = useState<PowerAction>("shutdown")
  const [gracefulShutdown, setGracefulShutdown] = useState<boolean>(false)
  const [preventSleep, setPreventSleep] = useState<boolean>(true)
  const [timerMode, setTimerMode] = useState<"timespan" | "time_of_day">("timespan")
  const [timerHours, setTimerHours] = useState<string>("0")
  const [timerMinutes, setTimerMinutes] = useState<string>("30")
  const [timerSeconds, setTimerSeconds] = useState<string>("0")
  const [targetTimeOfDay, setTargetTimeOfDay] = useState<string>("23:00")
  const [toolsError, setToolsError] = useState<string | null>(null)

  return (
    <div className="p-3.5 rounded-xl bg-[#0b1811] border border-emerald-900/70 space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-[10px] font-semibold text-emerald-400/80 uppercase tracking-wider">
          Configure Timer
        </span>
        {shutdownStatus.active && (
          <span className="text-[10px] font-medium text-amber-300 flex items-center gap-1 capitalize">
            <Timer className="h-3 w-3 animate-pulse" />
            {shutdownStatus.action || "shutdown"} active
          </span>
        )}
      </div>

      {shutdownStatus.active ? (
        <div className="py-4 text-center space-y-3">
          <div>
            <p className="text-xs text-emerald-400/75 capitalize">
              Scheduled {shutdownStatus.action || "shutdown"} in
            </p>
            <p className="text-3xl font-mono font-bold text-amber-300 tracking-wider mt-1">
              {formatCountdown(shutdownStatus.remaining_seconds)}
            </p>
          </div>
          <Button
            size="sm"
            onClick={() => {
              setToolsError(null)
              invoke<ShutdownTimerStatus>("cancel_shutdown_timer")
                .then((st) => onShutdownStatusChange(st))
                .catch((err) => setToolsError(String(err)))
            }}
            className="w-full h-8 text-xs bg-red-500/20 hover:bg-red-500/30 text-red-200 border border-red-500/40"
          >
            Cancel Timer
          </Button>
        </div>
      ) : (
        <div className={`space-y-2.5 ${disabled ? "opacity-50 pointer-events-none" : ""}`}>
          {/* Action Dropdown */}
          <div className="flex items-center justify-between gap-2">
            <span className="text-xs text-emerald-300/80 shrink-0">Action</span>
            <DropdownMenu.Root>
              <DropdownMenu.Trigger asChild>
                <button
                  type="button"
                  className="flex-1 max-w-[230px] h-8 px-2.5 rounded-lg bg-[#06100a] border border-emerald-900/80 hover:border-[#14ad72]/60 text-xs text-emerald-100 flex items-center justify-between transition-colors focus:outline-none focus:border-[#14ad72]"
                >
                  <span className="flex items-center gap-2">
                    {powerAction === "shutdown" && <Power className="h-3.5 w-3.5 text-[#14ad72]" />}
                    {powerAction === "restart" && <RotateCcw className="h-3.5 w-3.5 text-[#14ad72]" />}
                    {powerAction === "sleep" && <Moon className="h-3.5 w-3.5 text-[#14ad72]" />}
                    {powerAction === "lock" && <Lock className="h-3.5 w-3.5 text-[#14ad72]" />}
                    {powerAction === "logout" && <LogOut className="h-3.5 w-3.5 text-[#14ad72]" />}
                    <span className="capitalize">
                      {powerAction === "logout" ? "Log Out" : powerAction}
                    </span>
                  </span>
                  <ChevronDown className="h-3.5 w-3.5 text-emerald-400/70" />
                </button>
              </DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content
                  sideOffset={4}
                  align="end"
                  className="z-[100] w-[230px] rounded-xl bg-[#091811] border border-emerald-800/80 p-1 shadow-2xl text-emerald-100 animate-in fade-in-0 zoom-in-95"
                >
                  {(
                    [
                      { id: "shutdown", label: "Shutdown", Icon: Power },
                      { id: "restart", label: "Restart", Icon: RotateCcw },
                      { id: "sleep", label: "Sleep", Icon: Moon },
                      { id: "lock", label: "Lock", Icon: Lock },
                      { id: "logout", label: "Log Out", Icon: LogOut },
                    ] as const
                  ).map(({ id, label, Icon }) => (
                    <DropdownMenu.Item
                      key={id}
                      onSelect={() => setPowerAction(id)}
                      className={`flex items-center justify-between px-2.5 py-1.5 rounded-lg text-xs cursor-pointer outline-none transition-colors ${
                        powerAction === id
                          ? "bg-emerald-500/15 text-[#14ad72] font-medium"
                          : "text-emerald-100 hover:bg-emerald-900/40"
                      }`}
                    >
                      <span className="flex items-center gap-2">
                        <Icon className="h-3.5 w-3.5 text-[#14ad72]" />
                        <span>{label}</span>
                      </span>
                      {powerAction === id && <Check className="h-3.5 w-3.5 text-[#14ad72]" />}
                    </DropdownMenu.Item>
                  ))}
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          </div>

          {/* Switch Options */}
          <div className="space-y-1.5 pt-0.5">
            {(powerAction === "shutdown" || powerAction === "restart") && (
              <button
                type="button"
                onClick={() => setGracefulShutdown(!gracefulShutdown)}
                className="w-full flex items-center justify-between py-0.5 text-left"
              >
                <span className="text-[11px] text-emerald-200/85">
                  Graceful (do not force close apps)
                </span>
                <span
                  className={`inline-flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors ${
                    gracefulShutdown
                      ? "bg-[#14ad72]"
                      : "bg-[#06100a] border border-emerald-800"
                  }`}
                >
                  <span
                    className={`h-3 w-3 rounded-full transition-transform ${
                      gracefulShutdown
                        ? "translate-x-3 bg-slate-950"
                        : "translate-x-0 bg-emerald-400/60"
                    }`}
                  />
                </span>
              </button>
            )}

            <button
              type="button"
              onClick={() => setPreventSleep(!preventSleep)}
              className="w-full flex items-center justify-between py-0.5 text-left"
            >
              <span className="text-[11px] text-emerald-200/85">
                Prevent system from going to sleep
              </span>
              <span
                className={`inline-flex h-4 w-7 shrink-0 items-center rounded-full p-0.5 transition-colors ${
                  preventSleep ? "bg-[#14ad72]" : "bg-[#06100a] border border-emerald-800"
                }`}
              >
                <span
                  className={`h-3 w-3 rounded-full transition-transform ${
                    preventSleep
                      ? "translate-x-3 bg-slate-950"
                      : "translate-x-0 bg-emerald-400/60"
                  }`}
                />
              </span>
            </button>
          </div>

          {/* Schedule Mode */}
          <div className="pt-2 border-t border-emerald-900/50 space-y-2">
            <div className="grid grid-cols-2 gap-1 p-0.5 rounded-lg bg-[#06100a] border border-emerald-900/70">
              <button
                type="button"
                onClick={() => setTimerMode("timespan")}
                className={`py-1 rounded-md text-[11px] font-medium transition-colors ${
                  timerMode === "timespan"
                    ? "bg-emerald-500/20 text-[#14ad72] border border-[#14ad72]/40"
                    : "text-emerald-400/70 hover:text-emerald-200"
                }`}
              >
                After timespan
              </button>
              <button
                type="button"
                onClick={() => setTimerMode("time_of_day")}
                className={`py-1 rounded-md text-[11px] font-medium transition-colors ${
                  timerMode === "time_of_day"
                    ? "bg-emerald-500/20 text-[#14ad72] border border-[#14ad72]/40"
                    : "text-emerald-400/70 hover:text-emerald-200"
                }`}
              >
                At time of day
              </button>
            </div>

            {timerMode === "timespan" ? (
              <div className="space-y-2">
                <div className="grid grid-cols-4 gap-1.5">
                  {[
                    { label: "15m", h: "0", m: "15", s: "0" },
                    { label: "30m", h: "0", m: "30", s: "0" },
                    { label: "1h", h: "1", m: "0", s: "0" },
                    { label: "2h", h: "2", m: "0", s: "0" },
                  ].map((p) => (
                    <button
                      key={p.label}
                      type="button"
                      onClick={() => {
                        setTimerHours(p.h)
                        setTimerMinutes(p.m)
                        setTimerSeconds(p.s)
                      }}
                      className="py-1 rounded-md bg-[#06100a] border border-emerald-900/80 hover:border-[#14ad72]/60 text-[10.5px] font-semibold text-emerald-300 hover:text-[#14ad72] transition-colors"
                    >
                      {p.label}
                    </button>
                  ))}
                </div>

                <div className="grid grid-cols-3 gap-2">
                  {[
                    { label: "Hours", value: timerHours, setter: setTimerHours, max: 168 },
                    { label: "Minutes", value: timerMinutes, setter: setTimerMinutes, max: 59 },
                    { label: "Seconds", value: timerSeconds, setter: setTimerSeconds, max: 59 },
                  ].map((field) => {
                    const step = (delta: number) => {
                      const cur = parseInt(field.value || "0", 10) || 0
                      let next = cur + delta
                      if (next < 0) next = field.max
                      if (next > field.max) next = 0
                      field.setter(String(next))
                    }
                    return (
                      <div key={field.label}>
                        <span className="text-[10px] text-emerald-400/70 block mb-0.5">
                          {field.label}
                        </span>
                        <div className="flex items-center h-8 rounded-lg bg-[#06100a] border border-emerald-900/80 focus-within:border-[#14ad72] overflow-hidden">
                          <input
                            type="number"
                            min={0}
                            max={field.max}
                            value={field.value}
                            onChange={(e) => field.setter(e.target.value)}
                            className="w-full h-full px-2.5 bg-transparent text-xs font-mono text-emerald-100 focus:outline-none [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
                          />
                          <div className="flex flex-col border-l border-emerald-900/70 h-full w-5 shrink-0">
                            <button
                              type="button"
                              onClick={() => step(1)}
                              className="flex-1 flex items-center justify-center text-emerald-400/70 hover:text-[#14ad72] hover:bg-emerald-900/40 transition-colors"
                            >
                              <ChevronUp className="h-2.5 w-2.5" />
                            </button>
                            <button
                              type="button"
                              onClick={() => step(-1)}
                              className="flex-1 flex items-center justify-center border-t border-emerald-900/60 text-emerald-400/70 hover:text-[#14ad72] hover:bg-emerald-900/40 transition-colors"
                            >
                              <ChevronDown className="h-2.5 w-2.5" />
                            </button>
                          </div>
                        </div>
                      </div>
                    )
                  })}
                </div>
              </div>
            ) : (
              <div>
                <span className="text-[10px] text-emerald-400/70 block mb-1">
                  Target time (if earlier than now, schedules for tomorrow)
                </span>
                <input
                  type="time"
                  value={targetTimeOfDay}
                  onChange={(e) => setTargetTimeOfDay(e.target.value)}
                  className="w-full h-8 px-2.5 rounded-lg bg-[#06100a] border border-emerald-900/80 text-xs text-emerald-100 focus:outline-none focus:border-[#14ad72] [color-scheme:dark]"
                />
              </div>
            )}
          </div>

          {toolsError && (
            <div className="flex items-center gap-1.5 p-1.5 rounded-lg bg-red-950/80 border border-red-500/40 text-[10px] text-red-200">
              <AlertCircle className="h-3 w-3 text-red-400 shrink-0" />
              <span>{toolsError}</span>
            </div>
          )}

          <Button
            size="sm"
            onClick={() => {
              setToolsError(null)
              const h = Math.max(0, parseInt(timerHours || "0", 10) || 0)
              const m = Math.max(0, parseInt(timerMinutes || "0", 10) || 0)
              const s = Math.max(0, parseInt(timerSeconds || "0", 10) || 0)

              invoke<ShutdownTimerStatus>("start_shutdown_timer", {
                request: {
                  action: powerAction,
                  mode: timerMode,
                  hours: h,
                  minutes: m,
                  seconds: s,
                  target_time: targetTimeOfDay,
                  graceful: gracefulShutdown,
                  prevent_sleep: preventSleep,
                },
              })
                .then((st) => onShutdownStatusChange(st))
                .catch((err) => setToolsError(String(err)))
            }}
            className="w-full h-8 text-xs bg-[#14ad72] hover:bg-emerald-400 text-slate-950 font-semibold"
          >
            <Power className="h-3.5 w-3.5 mr-1.5" />
            Start Timer
          </Button>
        </div>
      )}
    </div>
  )
}

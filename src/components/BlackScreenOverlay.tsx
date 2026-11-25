import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"

export default function BlackScreenOverlay() {
  const [showHint, setShowHint] = useState(true)

  const handleExit = () => {
    invoke("stop_black_screen").catch(() => {})
  }

  useEffect(() => {
    let timer = setTimeout(() => setShowHint(false), 2000)

    let unlisten: (() => void) | undefined
    listen("black-screen-opened", () => {
      setShowHint(true)
      clearTimeout(timer)
      timer = setTimeout(() => setShowHint(false), 2000)
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(() => {})

    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault()
      handleExit()
    }

    window.addEventListener("keydown", onKeyDown)
    return () => {
      clearTimeout(timer)
      if (unlisten) unlisten()
      window.removeEventListener("keydown", onKeyDown)
    }
  }, [])

  return (
    <div
      onMouseDown={handleExit}
      className="fixed inset-0 w-screen h-screen bg-black cursor-none select-none flex items-center justify-center"
    >
      <div
        className={`pointer-events-none px-4 py-2 rounded-full bg-white/5 border border-white/10 text-white/50 text-xs tracking-wide transition-opacity duration-700 ${
          showHint ? "opacity-100" : "opacity-0"
        }`}
      >
        Press any key or click anywhere to exit Black Screen
      </div>
    </div>
  )
}

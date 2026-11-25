import { useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import {
  AlertCircle,
  Check,
  Film,
  FolderSearch,
  ImagePlus,
  Link2,
  Loader2,
  RotateCcw,
  Search,
  Sparkles,
} from "lucide-react"
import { Button } from "@/components/ui/button"
import type { PosterCandidate, SubfolderScanItem } from "@/types"

interface MoviePosterPanelProps {
  disabled?: boolean
}

export default function MoviePosterPanel({ disabled = false }: MoviePosterPanelProps) {
  const [folderPath, setFolderPath] = useState<string>("")
  const [query, setQuery] = useState<string>("")
  const [yearHint, setYearHint] = useState<string>("")
  const [candidates, setCandidates] = useState<PosterCandidate[]>([])
  const [subfolders, setSubfolders] = useState<SubfolderScanItem[]>([])
  const [loading, setLoading] = useState<boolean>(false)
  const [applyingUrl, setApplyingUrl] = useState<string | null>(null)
  const [batchStatus, setBatchStatus] = useState<string | null>(null)
  const [statusMsg, setStatusMsg] = useState<{ type: "ok" | "err"; text: string } | null>(null)
  const [customUrl, setCustomUrl] = useState<string>("")
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleLoadFolder = async (path: string) => {
    const clean = path.trim().replace(/^"|"$/g, "")
    if (!clean) return
    setFolderPath(clean)
    setStatusMsg(null)
    setSubfolders([])
    try {
      const [parsedTitle, parsedYear] = await invoke<[string, string | null]>(
        "parse_movie_folder_query",
        { folderPath: clean }
      )
      setQuery(parsedTitle)
      setYearHint(parsedYear || "")
      setLoading(true)
      const results = await invoke<PosterCandidate[]>("search_movie_posters", {
        query: parsedTitle,
        yearHint: parsedYear,
      })
      setCandidates(results)
      if (results.length === 0) {
        setStatusMsg({ type: "err", text: `No posters found for "${parsedTitle}". Try editing the title.` })
      }
    } catch (err) {
      setStatusMsg({ type: "err", text: String(err) })
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    const consumePending = () => {
      invoke<SubfolderScanItem[]>("take_pending_poster_folders")
        .then((items) => {
          if (!items || items.length === 0) return
          if (items.length === 1) {
            handleLoadFolder(items[0].path)
          } else {
            setCandidates([])
            setFolderPath(`${items.length} folders selected`)
            setSubfolders(items)
            setStatusMsg({
              type: "ok",
              text: `Loaded ${items.length} selected folders. Click "Auto-Apply Missing" or Pick individually.`,
            })
          }
        })
        .catch(() => {})
    }

    consumePending()

    let unlisten: (() => void) | undefined
    listen("movie-poster-folders-ready", () => {
      consumePending()
    })
      .then((fn) => {
        unlisten = fn
      })
      .catch(() => {})

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  const handleSearch = async () => {
    if (!query.trim()) return
    setLoading(true)
    setStatusMsg(null)
    setSubfolders([])
    try {
      const results = await invoke<PosterCandidate[]>("search_movie_posters", {
        query: query.trim(),
        yearHint: yearHint.trim() || null,
      })
      setCandidates(results)
      if (results.length === 0) {
        setStatusMsg({ type: "err", text: "No posters found. Try a shorter or different title." })
      }
    } catch (err) {
      setStatusMsg({ type: "err", text: String(err) })
    } finally {
      setLoading(false)
    }
  }

  const handleApply = async (posterUrl: string) => {
    if (!folderPath.trim()) {
      setStatusMsg({ type: "err", text: "Enter or paste the target movie folder path first." })
      return
    }
    setApplyingUrl(posterUrl)
    setStatusMsg(null)
    try {
      await invoke("apply_movie_poster", {
        folderPath: folderPath.trim().replace(/^"|"$/g, ""),
        posterUrl,
      })
      setStatusMsg({ type: "ok", text: "Poster applied! Explorer icon updated." })
    } catch (err) {
      setStatusMsg({ type: "err", text: String(err) })
    } finally {
      setApplyingUrl(null)
    }
  }

  const handleReset = async () => {
    if (!folderPath.trim()) return
    setStatusMsg(null)
    try {
      await invoke("reset_movie_poster", {
        folderPath: folderPath.trim().replace(/^"|"$/g, ""),
      })
      setStatusMsg({ type: "ok", text: "Folder icon reset to default." })
    } catch (err) {
      setStatusMsg({ type: "err", text: String(err) })
    }
  }

  const handleScanParent = async () => {
    const clean = folderPath.trim().replace(/^"|"$/g, "")
    if (!clean) {
      setStatusMsg({ type: "err", text: "Enter a parent Movies folder path to scan." })
      return
    }
    setLoading(true)
    setStatusMsg(null)
    setCandidates([])
    try {
      const list = await invoke<SubfolderScanItem[]>("scan_movie_subfolders", {
        parentPath: clean,
      })
      setSubfolders(list)
      if (list.length === 0) {
        setStatusMsg({ type: "err", text: "No subfolders found inside that directory." })
      }
    } catch (err) {
      setStatusMsg({ type: "err", text: String(err) })
    } finally {
      setLoading(false)
    }
  }

  const handleFilePicked = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    e.target.value = ""
    if (!file) return
    if (!folderPath.trim() || subfolders.length > 1) {
      setStatusMsg({ type: "err", text: "Select or enter a single target folder first." })
      return
    }
    const reader = new FileReader()
    reader.onload = () => {
      if (typeof reader.result === "string") {
        handleApply(reader.result)
      }
    }
    reader.readAsDataURL(file)
  }

  const handleBatchAutoApply = async () => {
    const pending = subfolders.filter((s) => !s.has_poster)
    if (pending.length === 0) {
      setStatusMsg({ type: "ok", text: "All subfolders already have posters!" })
      return
    }
    setLoading(true)
    setStatusMsg(null)
    let applied = 0
    for (let i = 0; i < pending.length; i++) {
      const item = pending[i]
      setBatchStatus(`Applying ${i + 1}/${pending.length}: ${item.name}`)
      try {
        const res = await invoke<PosterCandidate[]>("search_movie_posters", {
          query: item.parsed_query,
          yearHint: null,
        })
        if (res.length > 0) {
          await invoke("apply_movie_poster", {
            folderPath: item.path,
            posterUrl: res[0].poster_url,
          })
          applied++
          setSubfolders((prev) =>
            prev.map((s) => (s.path === item.path ? { ...s, has_poster: true } : s))
          )
        }
      } catch {
        // Continue with remaining folders
      }
    }
    setBatchStatus(null)
    setLoading(false)
    setStatusMsg({ type: "ok", text: `Applied posters to ${applied} of ${pending.length} folders.` })
  }

  const handleBatchResetAll = async () => {
    const withPosters = subfolders.filter((s) => s.has_poster)
    if (withPosters.length === 0) {
      setStatusMsg({ type: "ok", text: "No custom posters found in subfolders." })
      return
    }
    setLoading(true)
    setStatusMsg(null)
    let resetCount = 0
    for (let i = 0; i < withPosters.length; i++) {
      const item = withPosters[i]
      setBatchStatus(`Resetting ${i + 1}/${withPosters.length}: ${item.name}`)
      try {
        await invoke("reset_movie_poster", { folderPath: item.path })
        resetCount++
        setSubfolders((prev) =>
          prev.map((s) => (s.path === item.path ? { ...s, has_poster: false } : s))
        )
      } catch {
        // Continue
      }
    }
    setBatchStatus(null)
    setLoading(false)
    setStatusMsg({ type: "ok", text: `Reset ${resetCount} folder icons back to default.` })
  }

  return (
    <div className={`space-y-3 ${disabled ? "opacity-50 pointer-events-none" : ""}`}>
      {/* Folder Path + Actions */}
      <div className="p-3 rounded-xl bg-[#0b1811] border border-emerald-900/70 space-y-2">
        <span className="text-[10px] font-semibold text-emerald-400/80 uppercase tracking-wider block">
          Target Folder (or right-click a folder in Explorer)
        </span>
        <div className="flex items-center gap-1.5">
          <input
            type="text"
            value={folderPath}
            onChange={(e) => setFolderPath(e.target.value)}
            placeholder="e.g. D:\Movies\Inception (2010)"
            className="flex-1 h-8 px-2.5 rounded-lg bg-[#06100a] border border-emerald-900/80 text-xs text-emerald-100 placeholder:text-emerald-600/60 focus:outline-none focus:border-[#14ad72]"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={handleScanParent}
            disabled={loading}
            title="Scan subfolders inside this parent Movies folder"
            className="h-8 px-2.5 text-[11px] border-emerald-800 hover:bg-emerald-900/40 text-emerald-200 shrink-0"
          >
            <FolderSearch className="h-3.5 w-3.5 mr-1" />
            Scan Subfolders
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={handleReset}
            disabled={loading || !folderPath.trim()}
            title="Reset folder icon to default"
            className="h-8 px-2 text-[11px] border-emerald-800 hover:bg-red-500/15 hover:text-red-300 text-emerald-300 shrink-0"
          >
            <RotateCcw className="h-3.5 w-3.5" />
          </Button>
        </div>

        {/* Search Query + Year */}
        <div className="flex items-center gap-1.5 pt-1">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder="Movie title (e.g. Inception)"
            className="flex-1 h-8 px-2.5 rounded-lg bg-[#06100a] border border-emerald-900/80 text-xs text-emerald-100 placeholder:text-emerald-600/60 focus:outline-none focus:border-[#14ad72]"
          />
          <input
            type="text"
            value={yearHint}
            onChange={(e) => setYearHint(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleSearch()}
            placeholder="Year"
            className="w-16 h-8 px-2 rounded-lg bg-[#06100a] border border-emerald-900/80 text-xs text-emerald-100 placeholder:text-emerald-600/60 focus:outline-none focus:border-[#14ad72]"
          />
          <Button
            size="sm"
            onClick={handleSearch}
            disabled={loading || !query.trim()}
            className="h-8 px-3 text-xs bg-[#14ad72] hover:bg-emerald-400 text-slate-950 font-semibold shrink-0"
          >
            {loading ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <>
                <Search className="h-3.5 w-3.5 mr-1" />
                Find
              </>
            )}
          </Button>
        </div>

        {/* Custom Poster URL or Local File */}
        <div className="flex items-center gap-1.5 pt-1 border-t border-emerald-900/50">
          <div className="relative flex-1">
            <Link2 className="h-3 w-3 text-emerald-500/70 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              type="text"
              value={customUrl}
              onChange={(e) => setCustomUrl(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && customUrl.trim() && handleApply(customUrl)}
              placeholder="Or paste custom poster URL / local path..."
              className="w-full h-7 pl-7 pr-2 rounded-lg bg-[#06100a] border border-emerald-900/80 text-[11px] text-emerald-100 placeholder:text-emerald-600/60 focus:outline-none focus:border-[#14ad72]"
            />
          </div>
          <Button
            size="sm"
            variant="outline"
            onClick={() => customUrl.trim() && handleApply(customUrl)}
            disabled={loading || applyingUrl !== null || !customUrl.trim()}
            className="h-7 px-2.5 text-[10.5px] border-emerald-800 hover:bg-emerald-900/40 text-emerald-200 shrink-0"
          >
            Apply URL
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={handleFilePicked}
            className="hidden"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={() => fileInputRef.current?.click()}
            disabled={loading || applyingUrl !== null}
            title="Choose a poster image file from your PC"
            className="h-7 px-2.5 text-[10.5px] border-emerald-800 hover:bg-emerald-900/40 text-emerald-200 shrink-0"
          >
            <ImagePlus className="h-3 w-3 mr-1 text-[#14ad72]" />
            File
          </Button>
        </div>
      </div>

      {/* Status / Batch Progress */}
      {batchStatus && (
        <div className="flex items-center gap-2 p-2 rounded-lg bg-emerald-950/70 border border-[#14ad72]/40 text-xs text-emerald-200">
          <Loader2 className="h-3.5 w-3.5 animate-spin text-[#14ad72]" />
          <span className="truncate">{batchStatus}</span>
        </div>
      )}

      {statusMsg && (
        <div
          className={`flex items-center gap-1.5 p-2 rounded-lg text-[11px] border ${
            statusMsg.type === "ok"
              ? "bg-emerald-950/80 border-[#14ad72]/50 text-emerald-200"
              : "bg-red-950/80 border-red-500/40 text-red-200"
          }`}
        >
          {statusMsg.type === "ok" ? (
            <Check className="h-3.5 w-3.5 text-[#14ad72] shrink-0" />
          ) : (
            <AlertCircle className="h-3.5 w-3.5 text-red-400 shrink-0" />
          )}
          <span>{statusMsg.text}</span>
        </div>
      )}

      {/* Poster Results Grid */}
      {candidates.length > 0 && (
        <div className="grid grid-cols-4 gap-2">
          {candidates.map((c, idx) => (
            <button
              key={c.poster_url + idx}
              type="button"
              onClick={() => handleApply(c.poster_url)}
              disabled={applyingUrl !== null}
              className="group relative flex flex-col rounded-xl overflow-hidden bg-[#06100a] border border-emerald-900/70 hover:border-[#14ad72] transition-all text-left"
            >
              <div className="aspect-[2/3] w-full bg-black overflow-hidden">
                <img
                  src={c.poster_url}
                  alt={c.title}
                  className="w-full h-full object-cover group-hover:scale-105 transition-transform"
                />
              </div>
              <div className="p-1.5">
                <p className="text-[10.5px] font-semibold text-emerald-100 truncate">{c.title}</p>
                <p className="text-[9.5px] text-emerald-400/70">
                  {c.year || "—"} • {c.source}
                </p>
              </div>
              {applyingUrl === c.poster_url && (
                <div className="absolute inset-0 bg-black/75 flex items-center justify-center">
                  <Loader2 className="h-5 w-5 animate-spin text-[#14ad72]" />
                </div>
              )}
            </button>
          ))}
        </div>
      )}

      {/* Subfolder Batch List */}
      {subfolders.length > 0 && (
        <div className="p-3 rounded-xl bg-[#0b1811] border border-emerald-900/70 space-y-2">
          <div className="flex items-center justify-between gap-2">
            <span className="text-[10.5px] font-semibold text-emerald-300">
              Found {subfolders.length} subfolders
            </span>
            <div className="flex items-center gap-1.5">
              <Button
                size="sm"
                variant="outline"
                onClick={handleBatchResetAll}
                disabled={loading}
                className="h-7 px-2 text-[11px] border-emerald-800 hover:bg-red-500/15 hover:text-red-300 text-emerald-300"
              >
                <RotateCcw className="h-3 w-3 mr-1" />
                Reset All
              </Button>
              <Button
                size="sm"
                onClick={handleBatchAutoApply}
                disabled={loading}
                className="h-7 px-2.5 text-[11px] bg-[#14ad72] hover:bg-emerald-400 text-slate-950 font-semibold"
              >
                <Sparkles className="h-3 w-3 mr-1" />
                Auto-Apply Missing
              </Button>
            </div>
          </div>
          <div className="max-h-[150px] overflow-y-auto space-y-1 pr-1">
            {subfolders.map((s) => (
              <div
                key={s.path}
                className="flex items-center justify-between px-2 py-1.5 rounded-lg bg-[#06100a] border border-emerald-900/60 text-[11px]"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <Film
                    className={`h-3.5 w-3.5 shrink-0 ${
                      s.has_poster ? "text-[#14ad72]" : "text-emerald-600"
                    }`}
                  />
                  <span className="truncate text-emerald-100">{s.name}</span>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  {s.has_poster && (
                    <button
                      type="button"
                      onClick={async () => {
                        await invoke("reset_movie_poster", { folderPath: s.path }).catch(() => {})
                        setSubfolders((prev) =>
                          prev.map((x) => (x.path === s.path ? { ...x, has_poster: false } : x))
                        )
                      }}
                      className="text-[10px] text-red-400/80 hover:text-red-300 underline underline-offset-2"
                    >
                      Reset
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() => handleLoadFolder(s.path)}
                    className="text-[10px] text-emerald-400 hover:text-[#14ad72] underline underline-offset-2"
                  >
                    Pick
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

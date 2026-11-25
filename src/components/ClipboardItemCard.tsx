import type React from "react"
import {
  Check,
  Code2,
  Copy,
  Download,
  FileText,
  Image as ImageIcon,
  Link2,
  Maximize2,
  Pin,
  Pipette,
  Trash2,
} from "lucide-react"
import { Badge } from "@/components/ui/badge"
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { formatTimeAMPM } from "@/lib/formatters"
import type { ClipboardItem } from "@/types"

interface ClipboardItemCardProps {
  item: ClipboardItem
  index: number
  isSelected: boolean
  isCopied: boolean
  isSavedImage: boolean
  itemRef: (el: HTMLDivElement | null) => void
  onSelect: (index: number) => void
  onMouseEnter: (index: number) => void
  onPaste: (item: ClipboardItem) => void
  onCopyOnly: (item: ClipboardItem, e?: React.MouseEvent) => void
  onSaveImage: (item: ClipboardItem, e?: React.MouseEvent) => void
  onTogglePin: (id: string, e?: React.MouseEvent) => void
  onDelete: (id: string, e?: React.MouseEvent) => void
  onPreviewImage: (url: string) => void
}

export default function ClipboardItemCard({
  item,
  index,
  isSelected,
  isCopied,
  isSavedImage,
  itemRef,
  onSelect,
  onMouseEnter,
  onPaste,
  onCopyOnly,
  onSaveImage,
  onTogglePin,
  onDelete,
  onPreviewImage,
}: ClipboardItemCardProps) {
  const isImage = item.content_type === "image"
  const formattedTime = formatTimeAMPM(item.copied_at)
  const isHexColor = item.text_content && /^#[0-9A-Fa-f]{6}$/.test(item.text_content.trim())

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div
          ref={itemRef}
          onClick={() => {
            onSelect(index)
            onPaste(item)
          }}
          onMouseEnter={() => onMouseEnter(index)}
          className={`group relative flex flex-col gap-1.5 rounded-xl border p-2.5 transition-all cursor-pointer ${
            isSelected
              ? "border-emerald-500/60 bg-[#0d2218]/90 shadow-md ring-1 ring-emerald-500/30"
              : "border-emerald-900/40 bg-[#09150f]/60 hover:border-emerald-700/50 hover:bg-[#0c1d14]/80"
          }`}
        >
          {/* Top metadata row */}
          <div className="flex items-center justify-between text-[10px] text-emerald-400/70">
            <div className="flex items-center gap-1.5">
              {isImage ? (
                <>
                  <ImageIcon className="h-3.5 w-3.5 text-amber-400" />
                  <span className="capitalize font-medium text-amber-300">Image / Screenshot</span>
                  {item.image_width && item.image_height && (
                    <Badge variant="warning" className="h-3.5 px-1 font-mono">
                      {item.image_width}×{item.image_height}
                    </Badge>
                  )}
                </>
              ) : isHexColor ? (
                <>
                  <Pipette className="h-3.5 w-3.5 text-[#14ad72]" />
                  <span className="capitalize font-medium text-emerald-300">Color Hex</span>
                </>
              ) : item.content_type === "code" ? (
                <>
                  <Code2 className="h-3.5 w-3.5 text-emerald-400" />
                  <span className="capitalize font-medium text-emerald-300">Code</span>
                  {item.language && (
                    <Badge variant="accent" className="h-3.5 px-1 uppercase">
                      {item.language}
                    </Badge>
                  )}
                </>
              ) : item.content_type === "link" ? (
                <>
                  <Link2 className="h-3.5 w-3.5 text-teal-400" />
                  <span className="capitalize font-medium text-teal-300">Link</span>
                </>
              ) : (
                <>
                  <FileText className="h-3.5 w-3.5 text-emerald-400/80" />
                  <span className="capitalize font-medium text-emerald-300">Text</span>
                </>
              )}
            </div>

            {/* Right side: Metadata + Timestamp + Action Buttons */}
            <div className="flex items-center gap-1.5">
              {item.char_count && (
                <span className="text-[10px] text-emerald-500/70">{item.char_count} chars</span>
              )}
              <span className="text-[10px] text-emerald-500/80 font-mono">{formattedTime}</span>

              <div className="flex items-center gap-0.5 ml-0.5">
                {isImage && item.image_data_url && (
                  <button
                    onClick={(e) => onSaveImage(item, e)}
                    className={`p-1 rounded transition-colors ${
                      isSavedImage
                        ? "text-emerald-400 bg-emerald-900/60"
                        : "text-amber-400/80 hover:text-amber-300 hover:bg-amber-500/10"
                    }`}
                    title={isSavedImage ? "Saved to Downloads!" : "Save image to Downloads"}
                  >
                    {isSavedImage ? <Check className="h-3 w-3" /> : <Download className="h-3 w-3" />}
                  </button>
                )}

                <button
                  onClick={(e) => onTogglePin(item.id, e)}
                  className={`p-1 rounded hover:bg-emerald-900/60 transition-colors ${
                    item.is_pinned
                      ? "text-amber-400 fill-amber-400"
                      : "text-emerald-500/60 hover:text-amber-300"
                  }`}
                  title={item.is_pinned ? "Unpin Item (Ctrl+P)" : "Pin to Top (Ctrl+P)"}
                >
                  <Pin className={`h-3 w-3 ${item.is_pinned ? "fill-amber-400 text-amber-400" : ""}`} />
                </button>
                <button
                  onClick={(e) => onDelete(item.id, e)}
                  className="p-1 rounded text-emerald-500/60 hover:text-red-400 hover:bg-red-500/10 transition-colors"
                  title="Delete Item (Del)"
                >
                  <Trash2 className="h-3 w-3" />
                </button>
              </div>
            </div>
          </div>

          {/* Content Preview */}
          {isImage && item.image_data_url ? (
            <div className="relative rounded-lg overflow-hidden border border-emerald-900/50 bg-[#050c08] max-h-24 flex items-center justify-center group/img">
              <img
                src={item.image_data_url}
                alt="Copied clipboard content"
                className="max-h-24 w-full object-contain rounded-md"
              />
              <button
                onClick={(e) => {
                  e.stopPropagation()
                  onPreviewImage(item.image_data_url!)
                }}
                className="absolute top-1.5 right-1.5 p-1 rounded-md bg-[#0a1811]/90 hover:bg-[#0f241a] text-emerald-300 opacity-0 group-hover/img:opacity-100 transition-opacity"
                title="Expand preview"
              >
                <Maximize2 className="h-3 w-3" />
              </button>
            </div>
          ) : isHexColor ? (
            <div className="flex items-center gap-2 py-0.5">
              <span
                className="h-5 w-5 rounded-md border border-white/30 shadow-sm shrink-0"
                style={{ backgroundColor: item.text_content!.trim() }}
              />
              <span className="font-mono text-xs font-semibold text-emerald-100">
                {item.text_content!.trim()}
              </span>
            </div>
          ) : (
            <div className="text-xs text-emerald-100/90 font-normal line-clamp-2 break-all whitespace-pre-wrap font-sans select-text">
              {item.text_content}
            </div>
          )}

          {/* Copied feedback overlay */}
          {isCopied && (
            <div className="absolute inset-0 flex items-center justify-center rounded-xl bg-[#071f13]/90 border border-emerald-500/70 backdrop-blur-sm animate-in fade-in-50">
              <div className="flex items-center gap-1.5 text-xs font-semibold text-emerald-200">
                <Check className="h-4 w-4 text-[#14ad72]" />
                Copied to Clipboard!
              </div>
            </div>
          )}
        </div>
      </ContextMenuTrigger>

      <ContextMenuContent>
        <ContextMenuItem onClick={() => onPaste(item)}>
          <Check className="mr-2 h-3.5 w-3.5 text-[#14ad72]" />
          Paste into Input Field
        </ContextMenuItem>
        <ContextMenuItem onClick={(e) => onCopyOnly(item, e)}>
          <Copy className="mr-2 h-3.5 w-3.5 text-emerald-400" />
          Copy to Clipboard Only
        </ContextMenuItem>
        {isImage && item.image_data_url && (
          <ContextMenuItem onClick={(e) => onSaveImage(item, e)}>
            <Download className="mr-2 h-3.5 w-3.5 text-amber-400" />
            Save Image to Downloads
          </ContextMenuItem>
        )}
        <ContextMenuItem onClick={(e) => onTogglePin(item.id, e)}>
          <Pin className="mr-2 h-3.5 w-3.5 text-amber-400" />
          {item.is_pinned ? "Unpin Item" : "Pin to Top"}
        </ContextMenuItem>
        <ContextMenuSeparator />
        <ContextMenuItem
          onClick={(e) => onDelete(item.id, e)}
          className="text-red-400 focus:text-red-300 focus:bg-red-900/30"
        >
          <Trash2 className="mr-2 h-3.5 w-3.5" />
          Delete Item
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  )
}

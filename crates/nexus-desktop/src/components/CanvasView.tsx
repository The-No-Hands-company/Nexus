/**
 * CanvasView — block-based rich document editor for canvas channels.
 * Supports heading, paragraph, code, divider, image, callout, and table blocks.
 * Slash commands, drag-to-reorder, real-time collaboration via gateway events.
 *
 * v0.15 Community Ecosystem
 */
import { useEffect, useRef, useState, useCallback } from "react";
import { useStore, CanvasBlock, CanvasBlockType } from "../store";
import { invoke } from "../invoke";
import clsx from "clsx";

interface Props {
  channelId: string;
}

// ─── Block type menu ─────────────────────────────────────────────────────────

const BLOCK_MENU: { type: CanvasBlockType; icon: string; label: string; description: string }[] = [
  { type: "heading",   icon: "H",  label: "Heading",   description: "Large section title" },
  { type: "paragraph", icon: "¶",  label: "Paragraph", description: "Plain text content" },
  { type: "code",      icon: "<>", label: "Code",      description: "Syntax-highlighted code block" },
  { type: "divider",   icon: "─",  label: "Divider",   description: "Horizontal rule" },
  { type: "image",     icon: "🖼", label: "Image",     description: "Embed an image by URL" },
  { type: "callout",   icon: "💡", label: "Callout",   description: "Highlighted note or warning" },
  { type: "table",     icon: "⊞",  label: "Table",     description: "Simple 2-column key/value table" },
];

// ─── Block content defaults ───────────────────────────────────────────────────

function defaultContent(type: CanvasBlockType): Record<string, unknown> {
  switch (type) {
    case "heading":   return { text: "", level: 1 };
    case "paragraph": return { text: "" };
    case "code":      return { code: "", language: "plaintext" };
    case "divider":   return {};
    case "image":     return { url: "", alt: "" };
    case "callout":   return { text: "", emoji: "💡" };
    case "table":     return { rows: [["Key", "Value"]] };
  }
}

// ─── Individual block renderer/editor ─────────────────────────────────────────

interface BlockProps {
  block: CanvasBlock;
  editing: boolean;
  onEdit: () => void;
  onSave: (content: Record<string, unknown>) => void;
  onDelete: () => void;
  onDragStart: (e: React.DragEvent) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent) => void;
  dragging: boolean;
}

function BlockRow({ block, editing, onEdit, onSave, onDelete, onDragStart, onDragOver, onDrop, dragging }: BlockProps) {
  const [draft, setDraft] = useState<Record<string, unknown>>(block.content);

  useEffect(() => {
    setDraft(block.content);
  }, [block.content]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      setDraft(block.content);
      onEdit(); // toggle off
    }
    if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
      onSave(draft);
    }
  };

  const renderView = () => {
    const c = block.content;
    switch (block.blockType) {
      case "heading": {
        const level = (c.level as number) ?? 1;
        const Tag = (`h${Math.min(3, Math.max(1, level))}` as "h1" | "h2" | "h3");
        return (
          <Tag className={clsx("font-bold text-fg", level === 1 ? "text-xl" : level === 2 ? "text-lg" : "text-base")}>
            {(c.text as string) || <span className="text-muted/50 italic font-normal">Empty heading</span>}
          </Tag>
        );
      }
      case "paragraph":
        return (
          <p className="text-sm text-fg/90 leading-relaxed whitespace-pre-wrap">
            {(c.text as string) || <span className="text-muted/50 italic">Empty paragraph</span>}
          </p>
        );
      case "code":
        return (
          <div className="rounded-lg bg-bg-900/60 border border-bg-600/40 overflow-hidden">
            {c.language && (
              <div className="px-3 py-1 text-[10px] font-mono uppercase tracking-widest text-muted/70 border-b border-bg-600/30 bg-bg-800/40">
                {c.language as string}
              </div>
            )}
            <pre className="p-3 text-xs font-mono text-fg/90 overflow-x-auto leading-relaxed whitespace-pre">
              {(c.code as string) || "// empty"}
            </pre>
          </div>
        );
      case "divider":
        return <hr className="border-bg-600/50" />;
      case "image":
        return (c.url as string) ? (
          <figure className="space-y-1">
            <img src={c.url as string} alt={(c.alt as string) ?? ""} className="rounded-lg max-w-full max-h-96 object-contain border border-bg-600/30" />
            {c.alt && <figcaption className="text-xs text-muted">{c.alt as string}</figcaption>}
          </figure>
        ) : (
          <div className="rounded-lg border-2 border-dashed border-bg-600/50 p-6 text-center text-muted text-sm">
            🖼 No image URL set — click to edit
          </div>
        );
      case "callout": {
        const emoji = (c.emoji as string) || "💡";
        return (
          <div className="flex gap-3 rounded-lg bg-accent-900/20 border border-accent-700/30 p-3">
            <span className="text-xl shrink-0 leading-tight">{emoji}</span>
            <p className="text-sm text-fg/90 leading-relaxed whitespace-pre-wrap">
              {(c.text as string) || <span className="text-muted/50 italic">Empty callout</span>}
            </p>
          </div>
        );
      }
      case "table": {
        const rows = (c.rows as string[][]) ?? [];
        return (
          <div className="overflow-x-auto rounded-lg border border-bg-600/40">
            <table className="w-full text-sm">
              <tbody>
                {rows.map((row, ri) => (
                  <tr key={ri} className={ri === 0 ? "bg-bg-700/50 font-semibold text-fg" : "text-fg/80 hover:bg-bg-700/20 transition-colors"}>
                    {row.map((cell, ci) => (
                      <td key={ci} className="px-3 py-2 border-b border-r border-bg-600/30 last:border-r-0">
                        {cell}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      }
    }
  };

  const renderEditor = () => {
    switch (block.blockType) {
      case "heading":
        return (
          <div className="space-y-2">
            <div className="flex gap-2 text-xs text-muted items-center">
              <span>Level:</span>
              {[1, 2, 3].map((l) => (
                <button
                  key={l}
                  onClick={() => setDraft((d) => ({ ...d, level: l }))}
                  className={clsx("px-2 py-0.5 rounded transition-colors", (draft.level ?? 1) === l ? "bg-accent-600 text-white" : "bg-bg-600 hover:bg-bg-500")}
                >
                  H{l}
                </button>
              ))}
            </div>
            <input
              autoFocus
              value={(draft.text as string) ?? ""}
              onChange={(e) => setDraft((d) => ({ ...d, text: e.target.value }))}
              onKeyDown={handleKeyDown}
              className="w-full bg-bg-700/60 border border-bg-600/50 rounded-lg px-3 py-2 text-fg font-bold text-xl outline-none focus:border-accent-500/60"
              placeholder="Heading…"
            />
          </div>
        );
      case "paragraph":
        return (
          <textarea
            autoFocus
            rows={4}
            value={(draft.text as string) ?? ""}
            onChange={(e) => setDraft((d) => ({ ...d, text: e.target.value }))}
            onKeyDown={handleKeyDown}
            className="w-full bg-bg-700/60 border border-bg-600/50 rounded-lg px-3 py-2 text-sm text-fg outline-none focus:border-accent-500/60 resize-y leading-relaxed"
            placeholder="Write something…"
          />
        );
      case "code":
        return (
          <div className="space-y-2">
            <input
              value={(draft.language as string) ?? ""}
              onChange={(e) => setDraft((d) => ({ ...d, language: e.target.value }))}
              className="w-full bg-bg-700/60 border border-bg-600/50 rounded-lg px-3 py-1.5 text-xs font-mono text-fg outline-none focus:border-accent-500/60"
              placeholder="Language (e.g. rust, python, js)…"
            />
            <textarea
              autoFocus
              rows={6}
              value={(draft.code as string) ?? ""}
              onChange={(e) => setDraft((d) => ({ ...d, code: e.target.value }))}
              onKeyDown={handleKeyDown}
              className="w-full bg-bg-900/60 border border-bg-600/50 rounded-lg px-3 py-2 text-xs font-mono text-fg outline-none focus:border-accent-500/60 resize-y leading-relaxed"
              placeholder="// Code…"
            />
          </div>
        );
      case "divider":
        return (
          <div className="flex items-center gap-3 text-xs text-muted py-2">
            <hr className="flex-1 border-bg-600/50" />
            <span>Divider block (no settings)</span>
            <hr className="flex-1 border-bg-600/50" />
          </div>
        );
      case "image":
        return (
          <div className="space-y-2">
            <input
              autoFocus
              value={(draft.url as string) ?? ""}
              onChange={(e) => setDraft((d) => ({ ...d, url: e.target.value }))}
              onKeyDown={handleKeyDown}
              className="w-full bg-bg-700/60 border border-bg-600/50 rounded-lg px-3 py-2 text-sm text-fg outline-none focus:border-accent-500/60"
              placeholder="Image URL…"
            />
            <input
              value={(draft.alt as string) ?? ""}
              onChange={(e) => setDraft((d) => ({ ...d, alt: e.target.value }))}
              className="w-full bg-bg-700/60 border border-bg-600/50 rounded-lg px-3 py-2 text-sm text-fg outline-none focus:border-accent-500/60"
              placeholder="Alt text (optional)…"
            />
            {(draft.url as string) && (
              <img src={draft.url as string} alt="" className="rounded max-h-40 object-contain border border-bg-600/30 mt-1" />
            )}
          </div>
        );
      case "callout":
        return (
          <div className="space-y-2">
            <div className="flex gap-2 items-center">
              <input
                value={(draft.emoji as string) ?? "💡"}
                onChange={(e) => setDraft((d) => ({ ...d, emoji: e.target.value }))}
                className="w-14 bg-bg-700/60 border border-bg-600/50 rounded-lg px-2 py-1.5 text-center text-xl outline-none focus:border-accent-500/60"
                maxLength={4}
              />
              <span className="text-xs text-muted">Pick an emoji</span>
            </div>
            <textarea
              autoFocus
              rows={3}
              value={(draft.text as string) ?? ""}
              onChange={(e) => setDraft((d) => ({ ...d, text: e.target.value }))}
              onKeyDown={handleKeyDown}
              className="w-full bg-bg-700/60 border border-bg-600/50 rounded-lg px-3 py-2 text-sm text-fg outline-none focus:border-accent-500/60 resize-y"
              placeholder="Callout text…"
            />
          </div>
        );
      case "table": {
        const rows = (draft.rows as string[][]) ?? [["Key", "Value"]];
        return (
          <div className="space-y-2">
            <div className="overflow-x-auto rounded-lg border border-bg-600/40">
              <table className="w-full text-sm">
                <tbody>
                  {rows.map((row, ri) => (
                    <tr key={ri}>
                      {row.map((cell, ci) => (
                        <td key={ci} className="border-b border-r border-bg-600/30 last:border-r-0">
                          <input
                            value={cell}
                            onChange={(e) => {
                              const next = rows.map((r, rr) =>
                                rr === ri ? r.map((c, cc) => (cc === ci ? e.target.value : c)) : r
                              );
                              setDraft((d) => ({ ...d, rows: next }));
                            }}
                            className="w-full px-3 py-1.5 bg-transparent text-fg outline-none focus:bg-bg-700/30 text-sm"
                            placeholder={ri === 0 ? `Column ${ci + 1}` : `Value ${ci + 1}`}
                          />
                        </td>
                      ))}
                      <td className="border-b border-bg-600/30 w-6">
                        <button
                          onClick={() => setDraft((d) => ({ ...d, rows: rows.filter((_, rr) => rr !== ri) }))}
                          className="w-5 h-5 flex items-center justify-center text-muted hover:text-red-400 transition-colors text-xs"
                          title="Remove row"
                        >
                          ×
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <button
              onClick={() => setDraft((d) => ({ ...d, rows: [...rows, Array(rows[0]?.length ?? 2).fill("")] }))}
              className="text-xs text-accent-400 hover:text-accent-300 transition-colors"
            >
              + Add row
            </button>
          </div>
        );
      }
    }
  };

  return (
    <div
      draggable
      onDragStart={onDragStart}
      onDragOver={(e) => { e.preventDefault(); onDragOver(e); }}
      onDrop={onDrop}
      className={clsx(
        "group relative rounded-lg transition-all",
        dragging && "opacity-40 ring-2 ring-accent-500/60",
        !editing && "hover:bg-bg-700/20"
      )}
    >
      {/* Drag handle + actions */}
      <div className="absolute left-0 top-1/2 -translate-y-1/2 -translate-x-6 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col gap-0.5">
        <span
          className="cursor-grab text-muted hover:text-fg text-xs select-none px-0.5"
          title="Drag to reorder"
          draggable
          onDragStart={onDragStart}
        >
          ⣿
        </span>
      </div>

      {/* Action buttons */}
      <div className="absolute right-2 top-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1 z-10">
        {!editing && (
          <button
            onClick={onEdit}
            className="w-6 h-6 flex items-center justify-center rounded bg-bg-600/80 hover:bg-bg-500 text-muted hover:text-fg text-xs transition-colors"
            title="Edit block"
          >
            ✏
          </button>
        )}
        <button
          onClick={onDelete}
          className="w-6 h-6 flex items-center justify-center rounded bg-bg-600/80 hover:bg-red-700/80 text-muted hover:text-red-300 text-xs transition-colors"
          title="Delete block"
        >
          ×
        </button>
      </div>

      {/* Content */}
      <div className="px-2 py-1.5">
        {editing ? (
          <div className="space-y-2">
            {renderEditor()}
            <div className="flex gap-2 justify-end">
              <button
                onClick={() => { setDraft(block.content); onEdit(); }}
                className="text-xs text-muted hover:text-fg px-3 py-1.5 rounded bg-bg-600/60 hover:bg-bg-500 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={() => onSave(draft)}
                className="text-xs text-white px-3 py-1.5 rounded bg-accent-600 hover:bg-accent-500 transition-colors font-medium"
              >
                Save (Ctrl+↵)
              </button>
            </div>
          </div>
        ) : (
          <div onDoubleClick={onEdit} className="cursor-default min-h-[1.5rem]">
            {renderView()}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Main CanvasView component ────────────────────────────────────────────────

export default function CanvasView({ channelId }: Props) {
  const { canvasBlocks, loadCanvasBlocks, upsertCanvasBlock, removeCanvasBlock } = useStore();
  const blocks = canvasBlocks[channelId] ?? [];

  const [editingId, setEditingId] = useState<string | null>(null);
  const [showBlockMenu, setShowBlockMenu] = useState(false);
  const [saving, setSaving] = useState(false);
  const [dragOverId, setDragOverId] = useState<string | null>(null);
  const dragBlockId = useRef<string | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadCanvasBlocks(channelId);
  }, [channelId, loadCanvasBlocks]);

  // Close menu on outside click
  useEffect(() => {
    if (!showBlockMenu) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setShowBlockMenu(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showBlockMenu]);

  const generateId = () => crypto.randomUUID();

  const handleAddBlock = useCallback(async (type: CanvasBlockType) => {
    setShowBlockMenu(false);
    setSaving(true);
    const newBlock: CanvasBlock = {
      id: generateId(),
      channelId,
      blockType: type,
      content: defaultContent(type),
      position: (blocks[blocks.length - 1]?.position ?? 0) + 1000,
    };
    try {
      await invoke<CanvasBlock>("upsert_canvas_block", {
        channelId,
        blockId: newBlock.id,
        blockType: newBlock.blockType,
        content: newBlock.content,
        position: newBlock.position,
      });
      upsertCanvasBlock(channelId, newBlock);
      setEditingId(newBlock.id);
    } catch (e) {
      console.error("Failed to add block:", e);
    } finally {
      setSaving(false);
    }
  }, [blocks, channelId, upsertCanvasBlock]);

  const handleSaveBlock = useCallback(async (block: CanvasBlock, content: Record<string, unknown>) => {
    setSaving(true);
    const updated = { ...block, content };
    try {
      await invoke<CanvasBlock>("upsert_canvas_block", {
        channelId,
        blockId: block.id,
        blockType: block.blockType,
        content,
        position: block.position,
      });
      upsertCanvasBlock(channelId, updated);
      setEditingId(null);
    } catch (e) {
      console.error("Failed to save block:", e);
    } finally {
      setSaving(false);
    }
  }, [channelId, upsertCanvasBlock]);

  const handleDeleteBlock = useCallback(async (blockId: string) => {
    setSaving(true);
    try {
      await invoke("delete_canvas_block", { channelId, blockId });
      removeCanvasBlock(channelId, blockId);
      if (editingId === blockId) setEditingId(null);
    } catch (e) {
      console.error("Failed to delete block:", e);
    } finally {
      setSaving(false);
    }
  }, [channelId, editingId, removeCanvasBlock]);

  // ── Drag-to-reorder ──────────────────────────────────────────────────────────

  const handleDragStart = useCallback((blockId: string) => {
    dragBlockId.current = blockId;
  }, []);

  const handleDrop = useCallback(async (targetId: string) => {
    const srcId = dragBlockId.current;
    setDragOverId(null);
    dragBlockId.current = null;
    if (!srcId || srcId === targetId) return;

    const srcIndex = blocks.findIndex((b) => b.id === srcId);
    const tgtIndex = blocks.findIndex((b) => b.id === targetId);
    if (srcIndex === -1 || tgtIndex === -1) return;

    const reordered = [...blocks];
    const [moved] = reordered.splice(srcIndex, 1);
    reordered.splice(tgtIndex, 0, moved);

    // Reassign positions with even spacing
    const withPositions = reordered.map((b, i) => ({ ...b, position: (i + 1) * 1000 }));

    // Optimistic update
    withPositions.forEach((b) => upsertCanvasBlock(channelId, b));

    setSaving(true);
    try {
      await invoke("reorder_canvas_blocks", {
        channelId,
        blockOrder: withPositions.map((b) => ({ blockId: b.id, position: b.position })),
      });
    } catch (e) {
      console.error("Reorder failed:", e);
      // Reload to reconcile
      loadCanvasBlocks(channelId);
    } finally {
      setSaving(false);
    }
  }, [blocks, channelId, upsertCanvasBlock, loadCanvasBlocks]);

  return (
    <div className="flex flex-col h-full overflow-hidden bg-bg-800">
      {/* Canvas header */}
      <div className="h-12 px-4 flex items-center gap-2 border-b border-bg-600/50 shrink-0">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" className="text-accent-400 shrink-0">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-7 14H7v-2h5v2zm5-4H7v-2h10v2zm0-4H7V7h10v2z"/>
        </svg>
        <span className="font-semibold text-fg text-sm flex-1">Canvas</span>
        {saving && <span className="text-[10px] text-muted animate-pulse">Saving…</span>}
      </div>

      {/* Scrollable block list */}
      <div className="flex-1 overflow-y-auto px-8 py-6 max-w-3xl mx-auto w-full">
        {blocks.length === 0 ? (
          <div className="flex flex-col items-center justify-center gap-4 py-24 text-center">
            <div className="text-5xl select-none">📄</div>
            <div>
              <p className="text-lg font-semibold text-fg">Empty Canvas</p>
              <p className="text-sm text-muted mt-1">Start writing by adding a block below.</p>
            </div>
            <button
              onClick={() => setShowBlockMenu(true)}
              className="mt-2 flex items-center gap-2 rounded-lg bg-accent-600 hover:bg-accent-500 text-white text-sm px-5 py-2.5 font-medium transition-colors"
            >
              + Add a block
            </button>
          </div>
        ) : (
          <div className="space-y-2 relative pl-8">
            {blocks.map((block) => (
              <div
                key={block.id}
                onDragOver={() => setDragOverId(block.id)}
                onDrop={() => handleDrop(block.id)}
              >
                {dragOverId === block.id && dragBlockId.current !== block.id && (
                  <div className="h-0.5 bg-accent-500/70 rounded-full my-1" />
                )}
                <BlockRow
                  block={block}
                  editing={editingId === block.id}
                  onEdit={() => setEditingId((id) => (id === block.id ? null : block.id))}
                  onSave={(content) => handleSaveBlock(block, content)}
                  onDelete={() => handleDeleteBlock(block.id)}
                  onDragStart={(e) => { e.dataTransfer.effectAllowed = "move"; handleDragStart(block.id); }}
                  onDragOver={() => setDragOverId(block.id)}
                  onDrop={() => handleDrop(block.id)}
                  dragging={dragBlockId.current === block.id}
                />
              </div>
            ))}
          </div>
        )}

        {/* Add block button (bottom of list) */}
        {blocks.length > 0 && (
          <div className="mt-4 flex justify-center relative pl-8" ref={menuRef}>
            <button
              onClick={() => setShowBlockMenu((v) => !v)}
              className="flex items-center gap-1.5 text-xs text-muted hover:text-fg transition-colors rounded-lg px-3 py-2 hover:bg-bg-700/50 border border-dashed border-bg-600/40 hover:border-bg-600"
            >
              <span className="text-base leading-none">+</span>
              Add a block
            </button>

            {/* Block type dropdown */}
            {showBlockMenu && (
              <div className="absolute bottom-full mb-2 left-1/2 -translate-x-1/2 w-64 bg-bg-700 border border-bg-600/50 rounded-xl shadow-2xl overflow-hidden z-20">
                <p className="px-3 pt-2 pb-1 text-[10px] uppercase tracking-widest text-muted/70 font-medium">Add block</p>
                {BLOCK_MENU.map((bm) => (
                  <button
                    key={bm.type}
                    onClick={() => handleAddBlock(bm.type)}
                    className="w-full flex items-center gap-3 px-3 py-2 hover:bg-bg-600/60 transition-colors text-left"
                  >
                    <span className="w-7 h-7 flex items-center justify-center rounded bg-bg-600 text-xs font-mono text-fg/80 shrink-0">
                      {bm.icon}
                    </span>
                    <div className="min-w-0">
                      <p className="text-sm text-fg font-medium leading-tight">{bm.label}</p>
                      <p className="text-[10px] text-muted truncate">{bm.description}</p>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Floating add button for non-empty canvases — top-right */}
      {blocks.length > 0 && (
        <div className="absolute top-14 right-4 z-10" ref={blocks.length === 0 ? undefined : menuRef}>
          <div className="relative">
            <button
              onClick={() => setShowBlockMenu((v) => !v)}
              className="w-8 h-8 flex items-center justify-center rounded-full bg-accent-600 hover:bg-accent-500 text-white shadow-lg transition-colors text-sm font-bold"
              title="Add block"
            >
              +
            </button>
            {showBlockMenu && (
              <div className="absolute top-full right-0 mt-1 w-64 bg-bg-700 border border-bg-600/50 rounded-xl shadow-2xl overflow-hidden z-20">
                <p className="px-3 pt-2 pb-1 text-[10px] uppercase tracking-widest text-muted/70 font-medium">Add block</p>
                {BLOCK_MENU.map((bm) => (
                  <button
                    key={bm.type}
                    onClick={() => handleAddBlock(bm.type)}
                    className="w-full flex items-center gap-3 px-3 py-2 hover:bg-bg-600/60 transition-colors text-left"
                  >
                    <span className="w-7 h-7 flex items-center justify-center rounded bg-bg-600 text-xs font-mono text-fg/80 shrink-0">
                      {bm.icon}
                    </span>
                    <div className="min-w-0">
                      <p className="text-sm text-fg font-medium leading-tight">{bm.label}</p>
                      <p className="text-[10px] text-muted truncate">{bm.description}</p>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

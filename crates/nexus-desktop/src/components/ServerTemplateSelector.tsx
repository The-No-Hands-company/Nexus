import { useEffect } from "react";
import { useStore } from "../store";
import type { ServerTemplate } from "../store";

/**
 * ServerTemplateSelector — Renders template cards for use when creating a new server.
 * Emits the selected template's channel/role layout for the parent modal to apply.
 */
export default function ServerTemplateSelector({
  selectedId,
  onSelect,
}: {
  selectedId: string | null;
  onSelect: (template: ServerTemplate | null) => void;
}) {
  const { serverTemplates, loadServerTemplates } = useStore();

  useEffect(() => {
    loadServerTemplates();
  }, [loadServerTemplates]);

  const categoryIcons: Record<string, string> = {
    gaming: "🎮",
    education: "📚",
    teams: "💼",
    "open-source": "🔓",
    community: "🌐",
  };

  return (
    <div className="flex flex-col gap-3">
      <p className="text-sm text-gray-400">
        Start with a template or create from scratch.
      </p>

      {/* "Blank" option */}
      <button
        onClick={() => onSelect(null)}
        className={`text-left p-3 rounded-lg border transition-colors ${
          selectedId === null
            ? "border-indigo-500 bg-indigo-500/10"
            : "border-gray-700 bg-gray-800/50 hover:border-gray-600"
        }`}
      >
        <div className="flex items-center gap-2">
          <span className="text-lg">📝</span>
          <div>
            <p className="text-sm font-medium text-white">Blank Server</p>
            <p className="text-xs text-gray-500">Start from scratch</p>
          </div>
        </div>
      </button>

      {/* Template cards */}
      {serverTemplates.map((t) => (
        <button
          key={t.id}
          onClick={() => onSelect(t)}
          className={`text-left p-3 rounded-lg border transition-colors ${
            selectedId === t.id
              ? "border-indigo-500 bg-indigo-500/10"
              : "border-gray-700 bg-gray-800/50 hover:border-gray-600"
          }`}
        >
          <div className="flex items-center gap-2">
            <span className="text-lg">
              {t.iconUrl ? (
                <img src={t.iconUrl} alt="" className="w-6 h-6 rounded" />
              ) : (
                categoryIcons[t.category] ?? "📦"
              )}
            </span>
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-white truncate">{t.name}</p>
              {t.description && (
                <p className="text-xs text-gray-500 truncate">{t.description}</p>
              )}
            </div>
            <span className="text-[10px] text-gray-600 bg-gray-800 rounded px-1.5 py-0.5">
              {t.category}
            </span>
          </div>
        </button>
      ))}

      {serverTemplates.length === 0 && (
        <p className="text-xs text-gray-600 text-center py-4">No templates available</p>
      )}
    </div>
  );
}

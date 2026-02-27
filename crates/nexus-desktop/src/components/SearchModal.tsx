import { useState, useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "../invoke";
import { useStore } from "../store";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { formatDistanceToNow } from "date-fns";

interface SearchHit {
  id: string;
  channelId: string;
  authorId: string;
  authorUsername: string;
  content: string;
  createdAt: string;
}

interface SearchResult {
  query: string;
  totalHits: number | null;
  hits: SearchHit[];
}

interface Props {
  onClose: () => void;
}

export default function SearchModal({ onClose }: Props) {
  const navigate = useNavigate();
  const { setActiveChannel, channels } = useStore();
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const [totalHits, setTotalHits] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const dialogRef = useFocusTrap<HTMLDivElement>(true);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Close on backdrop click
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) onClose();
    },
    [onClose]
  );

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((s) => Math.min(s + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((s) => Math.max(s - 1, 0));
      } else if (e.key === "Enter" && results[selected]) {
        goToResult(results[selected]);
      }
    },
    [results, selected, onClose]
  );

    const goToResult = useCallback(
    (hit: SearchHit) => {
      setActiveChannel(hit.channelId);
      navigate(`/channel/${hit.channelId}`);
      onClose();
    },
    [setActiveChannel, navigate, onClose]
  );

  // Debounced search
  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!query.trim()) {
      setResults([]);
      setTotalHits(null);
      return;
    }
    debounceRef.current = setTimeout(async () => {
      setLoading(true);
      try {
        const res = await invoke<SearchResult>("search_messages", {
          q: query.trim(),
          limit: 20,
        });
        setResults(res.hits);
        setTotalHits(res.totalHits);
        setSelected(0);
      } catch (err) {
        console.error("search error", err);
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300);

    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [query]);

  const channelName = (channelId: string) =>
    channels.find((c) => c.id === channelId)?.name ?? channelId.slice(0, 8);

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/60 backdrop-blur-sm pt-[15vh]"
      onClick={handleBackdropClick}
      role="presentation"
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label="Search messages"
        className="w-full max-w-xl rounded-xl bg-bg-700 shadow-2xl overflow-hidden"
        onKeyDown={handleKeyDown}
      >
        {/* Search input */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-bg-600/50">
          {loading ? (
            <svg
              className="w-4 h-4 text-muted animate-spin shrink-0"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
              <path d="M12 2a10 10 0 0 1 10 10" />
            </svg>
          ) : (
            <svg
              className="w-4 h-4 text-muted shrink-0"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.35-4.35" />
            </svg>
          )}
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search messages…"
            aria-label="Search messages"
            aria-autocomplete="list"
            aria-controls="search-results"
            aria-expanded={results.length > 0}
            role="combobox"
            autoComplete="off"
            className="flex-1 bg-transparent text-sm text-fg placeholder:text-muted/60 outline-none"
          />
          <kbd className="text-[10px] text-muted border border-bg-600/60 rounded px-1.5 py-0.5 shrink-0">
            ESC
          </kbd>
        </div>

        {/* Results */}
        {results.length > 0 && (
          <div
            id="search-results"
            role="listbox"
            aria-label="Search results"
            className="max-h-80 overflow-y-auto"
          >
            {totalHits !== null && totalHits > results.length && (
              <p className="px-4 py-1.5 text-[11px] text-muted/70 border-b border-bg-600/30" aria-live="polite">
                Showing {results.length} of {totalHits} results
              </p>
            )}
            {results.map((hit, i) => (
              <button
                key={hit.id}
                role="option"
                aria-selected={i === selected}
                onClick={() => goToResult(hit)}
                onMouseEnter={() => setSelected(i)}
                className={`w-full flex gap-3 px-4 py-3 text-left transition-colors ${
                  i === selected
                    ? "bg-accent-600/20"
                    : "hover:bg-bg-600/40"
                }`}
              >
                {/* Avatar */}
                <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-bg-600/60 text-xs font-bold uppercase text-fg/70">
                  {hit.authorUsername.slice(0, 1)}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2 mb-0.5">
                    <span className="text-xs font-semibold text-fg">
                      {hit.authorUsername}
                    </span>
                    <span className="text-[10px] text-accent-400/80">
                      #{channelName(hit.channelId)}
                    </span>
                    <span className="ml-auto text-[10px] text-muted shrink-0">
                      {formatDistanceToNow(new Date(hit.createdAt), {
                        addSuffix: true,
                      })}
                    </span>
                  </div>
                  <p className="truncate text-xs text-fg/80 leading-relaxed">
                    {hit.content}
                  </p>
                </div>
              </button>
            ))}
          </div>
        )}

        {query.trim() && !loading && results.length === 0 && (
          <p className="px-4 py-6 text-center text-sm text-muted">
            No messages found for "{query}"
          </p>
        )}

        {!query.trim() && (
          <p className="px-4 py-4 text-xs text-muted/70">
            Type to search messages across all your servers and DMs.
          </p>
        )}

        {/* Footer hint */}
        <div className="flex items-center gap-4 border-t border-bg-600/30 px-4 py-2">
          <span className="flex items-center gap-1 text-[10px] text-muted">
            <kbd className="border border-bg-600/60 rounded px-1 py-px">↑↓</kbd>
            navigate
          </span>
          <span className="flex items-center gap-1 text-[10px] text-muted">
            <kbd className="border border-bg-600/60 rounded px-1 py-px">↵</kbd>
            jump
          </span>
          <span className="flex items-center gap-1 text-[10px] text-muted ml-auto">
            <kbd className="border border-bg-600/60 rounded px-1 py-px">Ctrl K</kbd>
            toggle
          </span>
        </div>
      </div>
    </div>
  );
}

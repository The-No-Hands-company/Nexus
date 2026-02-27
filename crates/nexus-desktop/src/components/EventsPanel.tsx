/**
 * EventsPanel — side panel showing scheduled server events.
 *
 * Shows upcoming and active events with RSVP support.
 * Users with MANAGE_EVENTS can create and cancel events.
 */

import { useState, useEffect } from "react";
import clsx from "clsx";
import { useStore, ServerEvent, ServerEventStatus } from "../store";
import { invoke } from "../invoke";
import { formatDistanceToNow, format, isPast } from "date-fns";

interface Props {
  serverId: string;
  onClose: () => void;
}

interface CreateEventForm {
  title: string;
  description: string;
  startsAt: string;
  endsAt: string;
  location: string;
}

const EMPTY_FORM: CreateEventForm = {
  title: "",
  description: "",
  startsAt: "",
  endsAt: "",
  location: "",
};

const STATUS_LABEL: Record<ServerEventStatus, string> = {
  scheduled: "Scheduled",
  active: "Live",
  completed: "Ended",
  cancelled: "Cancelled",
};

const STATUS_COLOR: Record<ServerEventStatus, string> = {
  scheduled: "text-blue-400",
  active: "text-green-400",
  completed: "text-muted",
  cancelled: "text-red-400",
};

export default function EventsPanel({ serverId, onClose }: Props) {
  const { serverEvents, loadServerEvents, upsertServerEvent, setEventsOpen } = useStore();
  const events = serverEvents[serverId] ?? [];
  const [filter, setFilter] = useState<"upcoming" | "past">("upcoming");
  const [creating, setCreating] = useState(false);
  const [form, setForm] = useState<CreateEventForm>(EMPTY_FORM);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadServerEvents(serverId);
  }, [serverId, loadServerEvents]);

  const displayed = events.filter((e) => {
    if (filter === "upcoming") return e.status === "scheduled" || e.status === "active";
    return e.status === "completed" || e.status === "cancelled";
  });

  const handleClose = () => {
    setEventsOpen(false);
    onClose();
  };

  const handleRsvp = async (event: ServerEvent) => {
    try {
      if (event.isInterested) {
        await invoke("un_rsvp_server_event", { serverId, eventId: event.id });
        upsertServerEvent(serverId, { ...event, interestedCount: Math.max(0, event.interestedCount - 1), isInterested: false });
      } else {
        await invoke("rsvp_server_event", { serverId, eventId: event.id });
        upsertServerEvent(serverId, { ...event, interestedCount: event.interestedCount + 1, isInterested: true });
      }
    } catch (e) {
      console.error("[EventsPanel] RSVP error", e);
    }
  };

  const handleCreate = async () => {
    if (!form.title.trim() || !form.startsAt) return;
    setSubmitting(true);
    setError(null);
    try {
      const ev = await invoke<ServerEvent>("create_server_event", {
        serverId,
        title: form.title.trim(),
        description: form.description.trim() || undefined,
        startsAt: new Date(form.startsAt).toISOString(),
        endsAt: form.endsAt ? new Date(form.endsAt).toISOString() : undefined,
        location: form.location.trim() || undefined,
      });
      upsertServerEvent(serverId, ev);
      setCreating(false);
      setForm(EMPTY_FORM);
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <aside className="flex h-full w-80 shrink-0 flex-col border-l border-bg-600/40 bg-bg-800">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-bg-600/40 px-4 py-3">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted">Server</p>
          <p className="text-sm font-medium text-fg">Events</p>
        </div>
        <button
          onClick={handleClose}
          className="flex h-6 w-6 items-center justify-center rounded text-muted hover:bg-bg-600/60 hover:text-fg"
          aria-label="Close events panel"
        >
          ✕
        </button>
      </div>

      {/* Filter tabs */}
      <div className="flex border-b border-bg-600/40 px-4 pt-2 gap-3">
        {(["upcoming", "past"] as const).map((f) => (
          <button
            key={f}
            onClick={() => setFilter(f)}
            className={clsx(
              "pb-2 text-xs font-medium transition-colors",
              filter === f
                ? "border-b-2 border-brand text-fg"
                : "text-muted hover:text-fg"
            )}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
      </div>

      {/* Event list */}
      <div className="flex-1 overflow-y-auto p-3 space-y-2">
        {displayed.length === 0 && (
          <p className="py-4 text-center text-sm text-muted">No {filter} events.</p>
        )}
        {displayed.map((ev) => (
          <div key={ev.id} className="rounded-xl bg-bg-700 p-3 space-y-2">
            <div className="flex items-start justify-between gap-2">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium text-fg">{ev.title}</p>
                <p className={clsx("text-xs font-medium", STATUS_COLOR[ev.status])}>
                  {STATUS_LABEL[ev.status]}
                </p>
              </div>
              {(ev.status === "scheduled" || ev.status === "active") && (
                <button
                  onClick={() => handleRsvp(ev)}
                  className={clsx(
                    "shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors",
                    ev.isInterested
                      ? "bg-brand/20 text-brand hover:bg-brand/30"
                      : "bg-bg-600/60 text-muted hover:bg-bg-600 hover:text-fg"
                  )}
                >
                  {ev.isInterested ? "✓ Interested" : "Interested"}
                </button>
              )}
            </div>
            <div className="space-y-0.5 text-xs text-muted">
              <p>🗓 {format(new Date(ev.startsAt), "MMM d, yyyy · h:mm a")}</p>
              {ev.location && <p>📍 {ev.location}</p>}
              {ev.interestedCount > 0 && (
                <p>❤️ {ev.interestedCount} interested</p>
              )}
            </div>
            {ev.description && (
              <p className="text-xs text-muted line-clamp-2">{ev.description}</p>
            )}
          </div>
        ))}
      </div>

      {/* Create event */}
      {creating ? (
        <div className="border-t border-bg-600/40 p-4 space-y-3">
          <p className="text-xs font-semibold text-fg uppercase tracking-widest">New Event</p>
          <input
            placeholder="Event title *"
            value={form.title}
            onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))}
            className="w-full rounded-lg bg-bg-700 px-3 py-2 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-brand"
          />
          <input
            type="datetime-local"
            value={form.startsAt}
            onChange={(e) => setForm((f) => ({ ...f, startsAt: e.target.value }))}
            className="w-full rounded-lg bg-bg-700 px-3 py-2 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-brand"
          />
          <input
            placeholder="Location (optional)"
            value={form.location}
            onChange={(e) => setForm((f) => ({ ...f, location: e.target.value }))}
            className="w-full rounded-lg bg-bg-700 px-3 py-2 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-brand"
          />
          <textarea
            placeholder="Description (optional)"
            value={form.description}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
            rows={2}
            className="w-full resize-none rounded-lg bg-bg-700 px-3 py-2 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-brand"
          />
          {error && <p className="text-xs text-red-400">{error}</p>}
          <div className="flex justify-end gap-2">
            <button
              onClick={() => { setCreating(false); setForm(EMPTY_FORM); }}
              className="rounded-lg bg-bg-700 px-3 py-1.5 text-sm text-muted hover:bg-bg-600/80 hover:text-fg"
            >
              Cancel
            </button>
            <button
              onClick={handleCreate}
              disabled={!form.title.trim() || !form.startsAt || submitting}
              className={clsx(
                "rounded-lg px-3 py-1.5 text-sm font-medium",
                form.title.trim() && form.startsAt && !submitting
                  ? "bg-brand text-white hover:opacity-90"
                  : "cursor-not-allowed bg-brand/40 text-white/50"
              )}
            >
              {submitting ? "Creating…" : "Create"}
            </button>
          </div>
        </div>
      ) : (
        <div className="border-t border-bg-600/40 p-4">
          <button
            onClick={() => setCreating(true)}
            className="w-full rounded-lg bg-brand px-4 py-2 text-sm font-medium text-white hover:opacity-90"
          >
            + New Event
          </button>
        </div>
      )}
    </aside>
  );
}

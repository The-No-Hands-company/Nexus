/**
 * CalendarPanel — server calendar for events.
 *
 * Shows upcoming events in a simple list view with RSVP controls,
 * creation form, and ICS export. Opened from the server sidebar.
 */

import { useEffect, useState } from "react";
import { useStore, CalendarEvent, CalendarRsvp, RsvpStatus } from "../store";
import { invoke } from "../invoke";
import clsx from "clsx";

interface Props {
  serverId: string;
  onClose: () => void;
}

const RSVP_LABELS: Record<RsvpStatus, string> = {
  going: "Going",
  interested: "Interested",
  declined: "Declined",
};

export default function CalendarPanel({ serverId, onClose }: Props) {
  const { calendarEvents, loadCalendarEvents, upsertCalendarEvent, removeCalendarEvent } = useStore();
  const events = calendarEvents[serverId] ?? [];

  const [creating, setCreating] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [location, setLocation] = useState("");
  const [startsAt, setStartsAt] = useState("");
  const [endsAt, setEndsAt] = useState("");
  const [allDay, setAllDay] = useState(false);
  const [rsvps, setRsvps] = useState<Record<string, CalendarRsvp[]>>({});

  useEffect(() => {
    loadCalendarEvents(serverId);
  }, [serverId, loadCalendarEvents]);

  const handleCreate = async () => {
    if (!title.trim() || !startsAt || !endsAt) return;
    try {
      const event = await invoke<CalendarEvent>("create_calendar_event", {
        serverId,
        title: title.trim(),
        description: description.trim() || undefined,
        location: location.trim() || undefined,
        startsAt,
        endsAt,
        allDay,
      });
      upsertCalendarEvent(serverId, event);
      resetForm();
    } catch (e) {
      console.error("create calendar event error", e);
    }
  };

  const resetForm = () => {
    setCreating(false);
    setTitle("");
    setDescription("");
    setLocation("");
    setStartsAt("");
    setEndsAt("");
    setAllDay(false);
  };

  const handleDelete = async (eventId: string) => {
    try {
      await invoke("delete_calendar_event", { serverId, eventId });
      removeCalendarEvent(serverId, eventId);
    } catch (e) {
      console.error("delete event error", e);
    }
  };

  const handleRsvp = async (eventId: string, status: RsvpStatus) => {
    try {
      const rsvp = await invoke<CalendarRsvp>("calendar_rsvp", { eventId, status });
      setRsvps((prev) => {
        const list = prev[eventId] ?? [];
        const without = list.filter((r) => r.userId !== rsvp.userId);
        return { ...prev, [eventId]: [...without, rsvp] };
      });
    } catch (e) {
      console.error("rsvp error", e);
    }
  };

  const loadRsvps = async (eventId: string) => {
    try {
      const list = await invoke<CalendarRsvp[]>("list_calendar_rsvps", { eventId });
      setRsvps((prev) => ({ ...prev, [eventId]: list }));
    } catch (e) {
      console.error("load rsvps error", e);
    }
  };

  const handleExportIcs = async () => {
    try {
      const ics = await invoke<string>("export_calendar_ics", { serverId });
      const blob = new Blob([ics], { type: "text/calendar" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "calendar.ics";
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("export ics error", e);
    }
  };

  const sortedEvents = [...events].sort(
    (a, b) => new Date(a.startsAt).getTime() - new Date(b.startsAt).getTime()
  );

  return (
    <aside className="flex h-full w-96 shrink-0 flex-col border-l border-bg-600/40 bg-bg-800">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-bg-600/40 px-4 py-3">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted">Calendar</p>
          <p className="text-sm font-medium text-fg">{events.length} events</p>
        </div>
        <div className="flex gap-2">
          <button
            onClick={handleExportIcs}
            className="rounded px-2 py-1 text-xs text-muted transition-colors hover:bg-bg-600/60 hover:text-fg"
            title="Export ICS"
          >
            📥
          </button>
          <button
            onClick={() => setCreating(!creating)}
            className="rounded bg-accent px-2 py-1 text-xs font-medium text-white hover:bg-accent/80"
          >
            + New
          </button>
          <button
            onClick={onClose}
            className="flex h-6 w-6 items-center justify-center rounded text-muted transition-colors hover:bg-bg-600/60 hover:text-fg"
          >
            ✕
          </button>
        </div>
      </div>

      {/* Create form */}
      {creating && (
        <div className="space-y-2 border-b border-bg-600/40 px-4 py-3">
          <input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Event title"
            className="w-full rounded bg-bg-700 px-3 py-1.5 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-accent"
          />
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Description (optional)"
            className="w-full rounded bg-bg-700 px-3 py-1.5 text-sm text-fg placeholder-muted outline-none"
          />
          <input
            value={location}
            onChange={(e) => setLocation(e.target.value)}
            placeholder="Location (optional)"
            className="w-full rounded bg-bg-700 px-3 py-1.5 text-sm text-fg placeholder-muted outline-none"
          />
          <div className="flex gap-2">
            <div className="flex-1">
              <label className="text-[10px] text-muted">Starts</label>
              <input
                type="datetime-local"
                value={startsAt}
                onChange={(e) => setStartsAt(e.target.value)}
                className="w-full rounded bg-bg-700 px-2 py-1 text-xs text-fg outline-none"
              />
            </div>
            <div className="flex-1">
              <label className="text-[10px] text-muted">Ends</label>
              <input
                type="datetime-local"
                value={endsAt}
                onChange={(e) => setEndsAt(e.target.value)}
                className="w-full rounded bg-bg-700 px-2 py-1 text-xs text-fg outline-none"
              />
            </div>
          </div>
          <label className="flex items-center gap-2 text-xs text-fg">
            <input
              type="checkbox"
              checked={allDay}
              onChange={(e) => setAllDay(e.target.checked)}
              className="rounded"
            />
            All day
          </label>
          <div className="flex justify-end gap-2">
            <button onClick={resetForm} className="rounded px-3 py-1 text-xs text-muted hover:text-fg">
              Cancel
            </button>
            <button
              onClick={handleCreate}
              disabled={!title.trim() || !startsAt || !endsAt}
              className="rounded bg-accent px-3 py-1 text-xs font-medium text-white disabled:opacity-40"
            >
              Create
            </button>
          </div>
        </div>
      )}

      {/* Event list */}
      <div className="flex-1 overflow-y-auto px-4 py-2">
        {sortedEvents.length === 0 ? (
          <p className="py-8 text-center text-sm text-muted">No events yet.</p>
        ) : (
          <ul className="space-y-2">
            {sortedEvents.map((event) => {
              const start = new Date(event.startsAt);
              const end = new Date(event.endsAt);
              const eventRsvps = rsvps[event.id] ?? [];

              return (
                <li key={event.id} className="rounded-md border border-bg-600/40 bg-bg-700/50 px-3 py-2">
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        {event.color && (
                          <span
                            className="inline-block h-2.5 w-2.5 rounded-full"
                            style={{ backgroundColor: event.color }}
                          />
                        )}
                        <span className="text-sm font-medium text-fg">{event.title}</span>
                      </div>
                      <p className="mt-0.5 text-[10px] text-muted">
                        {event.allDay
                          ? start.toLocaleDateString()
                          : `${start.toLocaleString()} — ${end.toLocaleString()}`}
                      </p>
                      {event.location && (
                        <p className="text-[10px] text-muted">📍 {event.location}</p>
                      )}
                      {event.description && (
                        <p className="mt-1 text-xs text-muted">{event.description}</p>
                      )}
                    </div>
                    <button
                      onClick={() => handleDelete(event.id)}
                      className="ml-2 text-xs text-muted hover:text-red-400"
                    >
                      ✕
                    </button>
                  </div>

                  {/* RSVP buttons */}
                  <div className="mt-2 flex gap-1">
                    {(["going", "interested", "declined"] as const).map((status) => (
                      <button
                        key={status}
                        onClick={() => handleRsvp(event.id, status)}
                        className={clsx(
                          "rounded px-2 py-0.5 text-[10px] transition-colors",
                          "text-muted hover:bg-bg-600/60 hover:text-fg"
                        )}
                      >
                        {RSVP_LABELS[status]}
                      </button>
                    ))}
                    <button
                      onClick={() => loadRsvps(event.id)}
                      className="ml-auto text-[10px] text-muted hover:text-fg"
                    >
                      {eventRsvps.length > 0 ? `${eventRsvps.length} responses` : "Load RSVPs"}
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </aside>
  );
}

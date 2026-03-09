/**
 * TaskBoard — task management panel for a channel.
 *
 * Displays tasks grouped by status, with inline creation, checklist editing,
 * and assignment controls. Rendered via channel view when channel type is "tasks"
 * or when toggled from the channel toolbar.
 */

import { useEffect, useState, useCallback } from "react";
import { useStore, Task, TaskStatus, TaskPriority, ChecklistItem } from "../store";
import { invoke } from "../invoke";
import clsx from "clsx";

interface Props {
  serverId: string;
  channelId: string;
}

const STATUS_LABELS: Record<TaskStatus, string> = {
  open: "Open",
  in_progress: "In Progress",
  done: "Done",
  cancelled: "Cancelled",
};

const PRIORITY_COLORS: Record<TaskPriority, string> = {
  low: "text-green-400",
  medium: "text-yellow-400",
  high: "text-orange-400",
  urgent: "text-red-400",
};

export default function TaskBoard({ serverId, channelId }: Props) {
  const { channelTasks, loadChannelTasks, upsertTask, removeTask } = useStore();
  const tasks = channelTasks[channelId] ?? [];

  const [newTitle, setNewTitle] = useState("");
  const [newPriority, setNewPriority] = useState<TaskPriority>("medium");
  const [expandedTaskId, setExpandedTaskId] = useState<string | null>(null);
  const [checklist, setChecklist] = useState<ChecklistItem[]>([]);
  const [newItemContent, setNewItemContent] = useState("");
  const [filterStatus, setFilterStatus] = useState<TaskStatus | "all">("all");

  useEffect(() => {
    loadChannelTasks(channelId, serverId);
  }, [channelId, serverId, loadChannelTasks]);

  const loadChecklist = useCallback(async (taskId: string) => {
    try {
      const items = await invoke<ChecklistItem[]>("list_checklist", { taskId });
      setChecklist(items);
    } catch (e) {
      console.error("loadChecklist error", e);
    }
  }, []);

  const handleCreate = async () => {
    if (!newTitle.trim()) return;
    try {
      const task = await invoke<Task>("create_task", {
        serverId,
        channelId,
        title: newTitle.trim(),
        priority: newPriority,
      });
      upsertTask(channelId, task);
      setNewTitle("");
    } catch (e) {
      console.error("create task error", e);
    }
  };

  const handleStatusChange = async (task: Task, status: TaskStatus) => {
    try {
      const updated = await invoke<Task>("update_task", {
        serverId,
        taskId: task.id,
        body: { status },
      });
      upsertTask(channelId, updated);
    } catch (e) {
      console.error("update task error", e);
    }
  };

  const handleDelete = async (taskId: string) => {
    try {
      await invoke("delete_task", { serverId, taskId });
      removeTask(channelId, taskId);
      if (expandedTaskId === taskId) setExpandedTaskId(null);
    } catch (e) {
      console.error("delete task error", e);
    }
  };

  const handleToggleExpand = (taskId: string) => {
    if (expandedTaskId === taskId) {
      setExpandedTaskId(null);
    } else {
      setExpandedTaskId(taskId);
      loadChecklist(taskId);
    }
  };

  const handleAddChecklistItem = async () => {
    if (!expandedTaskId || !newItemContent.trim()) return;
    try {
      const item = await invoke<ChecklistItem>("add_checklist_item", {
        taskId: expandedTaskId,
        content: newItemContent.trim(),
      });
      setChecklist((prev) => [...prev, item]);
      setNewItemContent("");
    } catch (e) {
      console.error("add checklist item error", e);
    }
  };

  const handleToggleChecklistItem = async (itemId: string) => {
    if (!expandedTaskId) return;
    try {
      const updated = await invoke<ChecklistItem>("toggle_checklist_item", {
        taskId: expandedTaskId,
        itemId,
      });
      setChecklist((prev) => prev.map((i) => (i.id === itemId ? updated : i)));
    } catch (e) {
      console.error("toggle checklist item error", e);
    }
  };

  const filtered = filterStatus === "all" ? tasks : tasks.filter((t) => t.status === filterStatus);

  return (
    <div className="flex h-full flex-col overflow-hidden bg-bg-800">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-bg-600/40 px-4 py-3">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-widest text-muted">Tasks</p>
          <p className="text-sm font-medium text-fg">{tasks.length} total</p>
        </div>
        <div className="flex gap-1">
          {(["all", "open", "in_progress", "done", "cancelled"] as const).map((s) => (
            <button
              key={s}
              onClick={() => setFilterStatus(s)}
              className={clsx(
                "rounded px-2 py-0.5 text-xs transition-colors",
                filterStatus === s ? "bg-accent text-white" : "text-muted hover:bg-bg-600/60 hover:text-fg"
              )}
            >
              {s === "all" ? "All" : STATUS_LABELS[s]}
            </button>
          ))}
        </div>
      </div>

      {/* New task form */}
      <div className="flex items-center gap-2 border-b border-bg-600/40 px-4 py-2">
        <input
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleCreate()}
          placeholder="New task…"
          className="flex-1 rounded bg-bg-700 px-3 py-1.5 text-sm text-fg placeholder-muted outline-none focus:ring-1 focus:ring-accent"
        />
        <select
          value={newPriority}
          onChange={(e) => setNewPriority(e.target.value as TaskPriority)}
          className="rounded bg-bg-700 px-2 py-1.5 text-xs text-fg outline-none"
        >
          <option value="low">Low</option>
          <option value="medium">Medium</option>
          <option value="high">High</option>
          <option value="urgent">Urgent</option>
        </select>
        <button
          onClick={handleCreate}
          disabled={!newTitle.trim()}
          className="rounded bg-accent px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-accent/80 disabled:opacity-40"
        >
          Add
        </button>
      </div>

      {/* Task list */}
      <div className="flex-1 overflow-y-auto px-4 py-2">
        {filtered.length === 0 ? (
          <p className="py-8 text-center text-sm text-muted">No tasks yet. Create one above!</p>
        ) : (
          <ul className="space-y-1.5">
            {filtered.map((task) => (
              <li key={task.id} className="rounded-md border border-bg-600/40 bg-bg-700/50">
                <div className="flex items-center gap-2 px-3 py-2">
                  {/* Status toggle */}
                  <button
                    onClick={() =>
                      handleStatusChange(task, task.status === "done" ? "open" : "done")
                    }
                    className={clsx(
                      "flex h-5 w-5 shrink-0 items-center justify-center rounded border text-xs transition-colors",
                      task.status === "done"
                        ? "border-green-500 bg-green-500/20 text-green-400"
                        : "border-bg-500 text-transparent hover:border-accent"
                    )}
                  >
                    ✓
                  </button>

                  {/* Title + priority */}
                  <button
                    onClick={() => handleToggleExpand(task.id)}
                    className={clsx(
                      "flex-1 text-left text-sm",
                      task.status === "done" ? "text-muted line-through" : "text-fg"
                    )}
                  >
                    <span className={clsx("mr-1.5 text-[10px] font-bold uppercase", PRIORITY_COLORS[task.priority])}>
                      {task.priority}
                    </span>
                    {task.title}
                  </button>

                  {/* Status selector */}
                  <select
                    value={task.status}
                    onChange={(e) => handleStatusChange(task, e.target.value as TaskStatus)}
                    className="rounded bg-bg-600 px-1.5 py-0.5 text-[10px] text-muted outline-none"
                  >
                    {Object.entries(STATUS_LABELS).map(([k, v]) => (
                      <option key={k} value={k}>{v}</option>
                    ))}
                  </select>

                  {/* Delete */}
                  <button
                    onClick={() => handleDelete(task.id)}
                    className="text-xs text-muted hover:text-red-400 transition-colors"
                  >
                    ✕
                  </button>
                </div>

                {/* Expanded checklist */}
                {expandedTaskId === task.id && (
                  <div className="border-t border-bg-600/40 px-4 py-2">
                    {task.description && (
                      <p className="mb-2 text-xs text-muted">{task.description}</p>
                    )}
                    <ul className="space-y-1">
                      {checklist.map((item) => (
                        <li key={item.id} className="flex items-center gap-2">
                          <button
                            onClick={() => handleToggleChecklistItem(item.id)}
                            className={clsx(
                              "flex h-4 w-4 shrink-0 items-center justify-center rounded border text-[10px] transition-colors",
                              item.checked
                                ? "border-green-500 bg-green-500/20 text-green-400"
                                : "border-bg-500 text-transparent hover:border-accent"
                            )}
                          >
                            ✓
                          </button>
                          <span className={clsx("text-xs", item.checked ? "text-muted line-through" : "text-fg")}>
                            {item.content}
                          </span>
                        </li>
                      ))}
                    </ul>
                    <div className="mt-2 flex items-center gap-1">
                      <input
                        value={newItemContent}
                        onChange={(e) => setNewItemContent(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleAddChecklistItem()}
                        placeholder="Add checklist item…"
                        className="flex-1 rounded bg-bg-600 px-2 py-1 text-xs text-fg placeholder-muted outline-none"
                      />
                      <button
                        onClick={handleAddChecklistItem}
                        disabled={!newItemContent.trim()}
                        className="rounded bg-accent/70 px-2 py-1 text-xs text-white disabled:opacity-40"
                      >
                        +
                      </button>
                    </div>
                    {task.dueAt && (
                      <p className="mt-1 text-[10px] text-muted">Due: {new Date(task.dueAt).toLocaleDateString()}</p>
                    )}
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

/**
 * RoleManagementPanel — full CRUD UI for server roles with permission matrix.
 *
 * Rendered inside the "Roles" tab of ServerSettingsModal.
 * Layout: role list (left) | edit / create / confirm-delete pane (right)
 */

import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import clsx from "clsx";

// ─── Types ───────────────────────────────────────────────────────────────────

interface Role {
  id: string;
  serverId: string;
  name: string;
  color: number | null;
  hoist: boolean;
  icon: string | null;
  position: number;
  permissions: number;
  mentionable: boolean;
  isDefault: boolean;
}

// ─── Permission definitions ───────────────────────────────────────────────────

const PERMISSION_GROUPS: {
  label: string;
  perms: { bit: number; label: string; desc: string }[];
}[] = [
  {
    label: "General",
    perms: [
      { bit: 2 ** 40, label: "Administrator", desc: "Overrides all other permissions" },
      { bit: 2 ** 0,  label: "View Channels",     desc: "See channels and read messages" },
      { bit: 2 ** 1,  label: "Manage Server",     desc: "Edit server settings" },
      { bit: 2 ** 2,  label: "Manage Channels",   desc: "Create, edit, and delete channels" },
      { bit: 2 ** 3,  label: "Manage Roles",      desc: "Edit roles below this one" },
      { bit: 2 ** 4,  label: "Create Invites",    desc: "Generate invite links" },
      { bit: 2 ** 5,  label: "Kick Members",      desc: "Remove members from the server" },
      { bit: 2 ** 6,  label: "Ban Members",       desc: "Ban and unban members" },
      { bit: 2 ** 7,  label: "View Audit Log",    desc: "View audit log entries" },
      { bit: 2 ** 8,  label: "Change Nickname",   desc: "Change own nickname" },
      { bit: 2 ** 9,  label: "Manage Nicknames",  desc: "Change others' nicknames" },
      { bit: 2 ** 10, label: "Manage Emojis",     desc: "Upload and delete custom emojis" },
      { bit: 2 ** 11, label: "Manage Webhooks",   desc: "Create and delete webhooks" },
    ],
  },
  {
    label: "Text",
    perms: [
      { bit: 2 ** 12, label: "Send Messages",          desc: "Post in text channels" },
      { bit: 2 ** 13, label: "Send in Threads",         desc: "Participate in threads" },
      { bit: 2 ** 14, label: "Create Public Threads",  desc: "Start public threads" },
      { bit: 2 ** 15, label: "Create Private Threads", desc: "Start private threads" },
      { bit: 2 ** 16, label: "Manage Threads",         desc: "Archive, delete, and edit threads" },
      { bit: 2 ** 17, label: "Embed Links",            desc: "Show link previews" },
      { bit: 2 ** 18, label: "Attach Files",           desc: "Upload files and images" },
      { bit: 2 ** 19, label: "Add Reactions",          desc: "React to messages" },
      { bit: 2 ** 20, label: "External Emojis",        desc: "Use emojis from other servers" },
      { bit: 2 ** 21, label: "Mention Everyone",       desc: "Ping @everyone and @here" },
      { bit: 2 ** 22, label: "Manage Messages",        desc: "Delete and pin messages" },
      { bit: 2 ** 23, label: "Read History",           desc: "Read message history" },
      { bit: 2 ** 24, label: "Use Commands",           desc: "Use slash commands and bots" },
    ],
  },
  {
    label: "Voice",
    perms: [
      { bit: 2 ** 25, label: "Connect",        desc: "Join voice channels" },
      { bit: 2 ** 26, label: "Speak",          desc: "Transmit audio" },
      { bit: 2 ** 27, label: "Video",          desc: "Use camera in voice" },
      { bit: 2 ** 28, label: "Mute Members",   desc: "Server-mute others" },
      { bit: 2 ** 29, label: "Deafen Members", desc: "Server-deafen others" },
      { bit: 2 ** 30, label: "Move Members",   desc: "Move users between channels" },
      { bit: 2 ** 31, label: "Voice Activity", desc: "Use voice activity detection" },
      { bit: 2 ** 32, label: "Screen Share",   desc: "Share screen in voice" },
    ],
  },
  {
    label: "Nexus Extras",
    perms: [
      { bit: 2 ** 34, label: "Record Voice",   desc: "Record with visible consent indicator" },
      { bit: 2 ** 35, label: "Manage Polls",   desc: "Create and manage polls" },
      { bit: 2 ** 36, label: "Manage Events",  desc: "Create scheduled activities" },
      { bit: 2 ** 37, label: "Pin Messages",   desc: "Pin messages in channels" },
      { bit: 2 ** 38, label: "Manage Plugins", desc: "Manage server plugins" },
      { bit: 2 ** 39, label: "View Analytics", desc: "View server insights" },
    ],
  },
];

// ─── Permission helpers (arithmetic — safe for >32-bit values) ───────────────

/** Returns true if the power-of-2 `bit` is set in `perms`. */
function hasPerm(perms: number, bit: number): boolean {
  return Math.trunc(perms / bit) % 2 === 1;
}

/** Toggle a single power-of-2 permission bit in `perms`. */
function togglePerm(perms: number, bit: number): number {
  return hasPerm(perms, bit) ? perms - bit : perms + bit;
}

// ─── Color helpers ────────────────────────────────────────────────────────────

const COLOR_PRESETS: (number | null)[] = [
  null,
  0x1abc9c, 0x2ecc71, 0x3498db, 0x9b59b6, 0xe91e63,
  0xf1c40f, 0xe67e22, 0xe74c3c, 0x95a5a6, 0x607d8b,
  0x11806a, 0x1f8b4c, 0x206694, 0x71368a, 0xad1457,
  0xc27c0e, 0xa84300, 0x992d22, 0x979c9f, 0x546e7a,
];

function colorToHex(color: number): string {
  return "#" + color.toString(16).padStart(6, "0");
}

function hexToColor(hex: string): number {
  return parseInt(hex.replace("#", ""), 16);
}

// ─── Component ────────────────────────────────────────────────────────────────

interface Props {
  serverId: string;
}

interface EditState {
  name: string;
  color: number | null;
  permissions: number;
  hoist: boolean;
  mentionable: boolean;
}

export default function RoleManagementPanel({ serverId }: Props) {
  const [roles, setRoles] = useState<Role[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Editing state
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [edit, setEdit] = useState<EditState | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveOk, setSaveOk] = useState(false);

  // Delete confirmation
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  // Create new role
  const [creating, setCreating] = useState(false);
  const [newRoleName, setNewRoleName] = useState("");
  const [createBusy, setCreateBusy] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // ── Load ──────────────────────────────────────────────────────────────────

  const loadRoles = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await invoke<Role[]>("list_roles", { serverId });
      setRoles(data.slice().sort((a, b) => b.position - a.position));
    } catch (e) {
      setLoadError(String(e));
    } finally {
      setLoading(false);
    }
  }, [serverId]);

  useEffect(() => {
    loadRoles();
  }, [loadRoles]);

  // ── Select a role to edit ─────────────────────────────────────────────────

  const selectRole = (role: Role) => {
    setSelectedId(role.id);
    setEdit({
      name: role.name,
      color: role.color,
      permissions: role.permissions,
      hoist: role.hoist,
      mentionable: role.mentionable,
    });
    setSaveError(null);
    setSaveOk(false);
    setCreating(false);
    setConfirmDeleteId(null);
    setDeleteError(null);
  };

  // ── Save edits ────────────────────────────────────────────────────────────

  const handleSave = async () => {
    if (!selectedId || !edit) return;
    setSaving(true);
    setSaveError(null);
    setSaveOk(false);
    try {
      await invoke("update_role", {
        serverId,
        roleId: selectedId,
        name: edit.name || undefined,
        color: edit.color,
        permissions: edit.permissions,
        hoist: edit.hoist,
        mentionable: edit.mentionable,
      });
      setSaveOk(true);
      await loadRoles();
    } catch (e) {
      setSaveError(String(e));
    } finally {
      setSaving(false);
    }
  };

  // ── Delete ────────────────────────────────────────────────────────────────

  const handleDelete = async (roleId: string) => {
    setDeleting(true);
    setDeleteError(null);
    try {
      await invoke("delete_role", { serverId, roleId });
      if (selectedId === roleId) {
        setSelectedId(null);
        setEdit(null);
      }
      setConfirmDeleteId(null);
      await loadRoles();
    } catch (e) {
      setDeleteError(String(e));
    } finally {
      setDeleting(false);
    }
  };

  // ── Create ────────────────────────────────────────────────────────────────

  const handleCreate = async () => {
    if (!newRoleName.trim()) return;
    setCreateBusy(true);
    setCreateError(null);
    try {
      const role = await invoke<Role>("create_role", {
        serverId,
        name: newRoleName.trim(),
        color: null,
        permissions: null,
        hoist: null,
        mentionable: null,
      });
      await loadRoles();
      setCreating(false);
      setNewRoleName("");
      selectRole(role);
    } catch (e) {
      setCreateError(String(e));
    } finally {
      setCreateBusy(false);
    }
  };

  const cancelCreate = () => {
    setCreating(false);
    setNewRoleName("");
    setCreateError(null);
  };

  const startDelete = (role: Role) => {
    setConfirmDeleteId(role.id);
    setSelectedId(null);
    setEdit(null);
    setSaveError(null);
    setDeleteError(null);
  };

  // ── Derived ───────────────────────────────────────────────────────────────

  const selectedRole = roles.find((r) => r.id === selectedId) ?? null;
  const confirmRole = roles.find((r) => r.id === confirmDeleteId) ?? null;

  return (
    <div className="flex gap-5">
      {/* ── Role list ────────────────────────────────────────────────────── */}
      <div className="w-40 shrink-0 flex flex-col">
        <p className="mb-2 text-[10px] font-semibold uppercase tracking-widest text-muted/70">
          Roles · {roles.length}
        </p>

        {loading && <p className="text-xs text-muted">Loading…</p>}
        {loadError && <p className="text-xs text-red-400">{loadError}</p>}

        <div className="space-y-0.5">
          {roles.map((role) => (
            <div key={role.id} className="group flex items-center gap-1">
              <button
                onClick={() => selectRole(role)}
                className={clsx(
                  "flex-1 flex items-center gap-1.5 rounded px-2 py-1 text-left text-sm truncate transition-colors",
                  selectedId === role.id
                    ? "bg-bg-600/60 text-fg font-medium"
                    : "text-muted hover:bg-bg-600/40 hover:text-fg"
                )}
              >
                <span
                  className="inline-block h-2.5 w-2.5 shrink-0 rounded-full"
                  style={{
                    background: role.color != null ? colorToHex(role.color) : "#5d6a75",
                  }}
                />
                <span className="truncate">{role.name}</span>
              </button>

              {/* Delete button — hidden until hover, disabled for @everyone */}
              {!role.isDefault && (
                <button
                  aria-label={`Delete ${role.name}`}
                  onClick={() => startDelete(role)}
                  className="opacity-0 group-hover:opacity-100 shrink-0 rounded p-0.5 text-muted hover:text-red-400 transition-all"
                >
                  <svg
                    width="11"
                    height="11"
                    viewBox="0 0 16 16"
                    fill="currentColor"
                    aria-hidden="true"
                  >
                    <path d="M5.5 5.5A.5.5 0 0 1 6 6v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm2.5 0a.5.5 0 0 1 .5.5v6a.5.5 0 0 1-1 0V6a.5.5 0 0 1 .5-.5zm3 .5a.5.5 0 0 0-1 0v6a.5.5 0 0 0 1 0V6z" />
                    <path
                      fillRule="evenodd"
                      d="M14.5 3a1 1 0 0 1-1 1H13v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V4h-.5a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1H6a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1h3.5a1 1 0 0 1 1 1v1zM4.118 4 4 4.059V13a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1V4.059L11.882 4H4.118zM2.5 3V2h11v1h-11z"
                    />
                  </svg>
                </button>
              )}
            </div>
          ))}
        </div>

        {/* Create inline */}
        {creating ? (
          <div className="mt-3 space-y-1.5">
            <input
              autoFocus
              value={newRoleName}
              onChange={(e) => setNewRoleName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
                if (e.key === "Escape") cancelCreate();
              }}
              placeholder="Role name…"
              maxLength={100}
              className="w-full rounded border border-bg-600/50 bg-bg-700 px-2 py-1 text-xs text-fg placeholder:text-muted/60 outline-none focus:border-accent-500"
            />
            {createError && <p className="text-[10px] text-red-400">{createError}</p>}
            <div className="flex gap-1">
              <button
                onClick={handleCreate}
                disabled={createBusy || !newRoleName.trim()}
                className="flex-1 rounded bg-accent-600 px-2 py-0.5 text-xs text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
              >
                {createBusy ? "…" : "Create"}
              </button>
              <button
                onClick={cancelCreate}
                className="rounded px-2 py-0.5 text-xs text-muted hover:text-fg transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <button
            onClick={() => {
              setCreating(true);
              setSelectedId(null);
              setEdit(null);
              setConfirmDeleteId(null);
            }}
            className="mt-3 w-full rounded px-2 py-1 text-left text-xs text-accent-400 hover:bg-bg-600/40 hover:text-accent-300 transition-colors"
          >
            + Create Role
          </button>
        )}
      </div>

      {/* ── Right pane ───────────────────────────────────────────────────── */}
      <div className="flex-1 min-w-0">
        {/* Placeholder when nothing selected */}
        {!edit && !confirmRole && (
          <div className="flex h-32 items-center justify-center">
            <p className="text-sm text-muted">
              {!loading && roles.length === 0
                ? "No roles yet — create one to get started."
                : "Select a role to edit its settings and permissions."}
            </p>
          </div>
        )}

        {/* ── Delete confirmation ─────────────────────────────────────────── */}
        {confirmRole && (
          <div className="space-y-4">
            <h3 className="text-base font-semibold text-fg">
              Delete "{confirmRole.name}"?
            </h3>
            <p className="text-sm text-muted">
              This will permanently delete the role and remove it from all members. This cannot
              be undone.
            </p>
            {deleteError && <p className="text-xs text-red-400">{deleteError}</p>}
            <div className="flex gap-2">
              <button
                onClick={() => handleDelete(confirmRole.id)}
                disabled={deleting}
                className="rounded-lg bg-red-700 px-4 py-1.5 text-sm font-medium text-white hover:bg-red-600 disabled:opacity-40 transition-colors"
              >
                {deleting ? "Deleting…" : "Delete Role"}
              </button>
              <button
                onClick={() => {
                  setConfirmDeleteId(null);
                  setDeleteError(null);
                }}
                className="rounded-lg border border-bg-600/50 px-4 py-1.5 text-sm text-muted hover:text-fg transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        )}

        {/* ── Edit panel ─────────────────────────────────────────────────── */}
        {edit && selectedRole && (
          <div className="space-y-5">
            {/* Header */}
            <div className="flex items-center gap-2.5">
              <span
                className="inline-block h-4 w-4 shrink-0 rounded-full"
                style={{ background: edit.color != null ? colorToHex(edit.color) : "#5d6a75" }}
              />
              <h3 className="text-base font-semibold text-fg truncate">{selectedRole.name}</h3>
              {selectedRole.isDefault && (
                <span className="rounded bg-bg-600/60 px-1.5 py-0.5 text-[10px] text-muted">
                  default
                </span>
              )}
            </div>

            {/* Name */}
            <div>
              <label className="mb-1 block text-xs font-medium uppercase tracking-widest text-muted">
                Role Name
              </label>
              <input
                value={edit.name}
                onChange={(e) => setEdit((p) => p && { ...p, name: e.target.value })}
                disabled={selectedRole.isDefault}
                maxLength={100}
                className="w-full rounded-lg border border-bg-600/50 bg-bg-700 px-3 py-2 text-sm text-fg placeholder:text-muted/60 outline-none focus:border-accent-500 disabled:opacity-50"
              />
              {selectedRole.isDefault && (
                <p className="mt-1 text-[11px] text-muted/60">
                  The @everyone role name cannot be changed.
                </p>
              )}
            </div>

            {/* Color */}
            <div>
              <label className="mb-2 block text-xs font-medium uppercase tracking-widest text-muted">
                Role Color
              </label>
              <div className="flex flex-wrap items-center gap-1.5">
                {COLOR_PRESETS.map((preset, i) => (
                  <button
                    key={i}
                    aria-label={preset == null ? "No color" : colorToHex(preset)}
                    onClick={() => setEdit((p) => p && { ...p, color: preset })}
                    className={clsx(
                      "h-5 w-5 rounded-full border-2 transition-transform hover:scale-110",
                      edit.color === preset ? "border-white scale-110" : "border-transparent"
                    )}
                    style={{ background: preset != null ? colorToHex(preset) : "#5d6a75" }}
                  />
                ))}
                {/* Custom hex */}
                <div className="flex items-center gap-0.5 ml-1">
                  <span className="text-xs text-muted">#</span>
                  <input
                    type="text"
                    maxLength={6}
                    placeholder="hex…"
                    value={
                      edit.color != null ? edit.color.toString(16).padStart(6, "0") : ""
                    }
                    onChange={(e) => {
                      const v = e.target.value.replace(/[^0-9a-fA-F]/g, "");
                      if (v.length === 6) setEdit((p) => p && { ...p, color: hexToColor(v) });
                      else if (v.length === 0) setEdit((p) => p && { ...p, color: null });
                    }}
                    className="w-16 rounded border border-bg-600/50 bg-bg-700 px-1.5 py-0.5 font-mono text-xs text-fg outline-none focus:border-accent-500"
                  />
                </div>
              </div>
            </div>

            {/* Hoist + Mentionable toggles */}
            <div className="flex flex-wrap gap-6">
              {(
                [
                  {
                    key: "hoist" as const,
                    label: "Display separately",
                    desc: "Show this role's members separately in the member list",
                  },
                  {
                    key: "mentionable" as const,
                    label: "Mentionable",
                    desc: "Allow anyone to @mention this role",
                  },
                ] satisfies { key: keyof EditState; label: string; desc: string }[]
              ).map(({ key, label, desc }) => (
                <label key={key} className="flex cursor-pointer items-start gap-2">
                  <button
                    role="switch"
                    aria-checked={!!edit[key]}
                    onClick={() => setEdit((p) => p && { ...p, [key]: !p[key] })}
                    className={clsx(
                      "relative mt-0.5 h-4 w-7 shrink-0 rounded-full transition-colors",
                      edit[key] ? "bg-accent-600" : "bg-bg-600/60"
                    )}
                  >
                    <span
                      className={clsx(
                        "absolute top-0.5 h-3 w-3 rounded-full bg-white shadow transition-transform",
                        edit[key] ? "translate-x-3.5" : "translate-x-0.5"
                      )}
                    />
                  </button>
                  <div>
                    <div className="text-sm text-fg">{label}</div>
                    <div className="text-[11px] text-muted/70">{desc}</div>
                  </div>
                </label>
              ))}
            </div>

            {/* Permission matrix */}
            <div>
              <p className="mb-3 text-xs font-medium uppercase tracking-widest text-muted">
                Permissions
              </p>
              <div className="space-y-5">
                {PERMISSION_GROUPS.map((group) => (
                  <div key={group.label}>
                    <p className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted/60">
                      {group.label}
                    </p>
                    <div className="space-y-0.5">
                      {group.perms.map(({ bit, label, desc }) => {
                        const active = hasPerm(edit.permissions, bit);
                        return (
                          <div
                            key={bit}
                            className="flex items-center justify-between gap-3 rounded px-2 py-1 hover:bg-bg-600/25 transition-colors"
                          >
                            <div>
                              <span className="text-sm text-fg">{label}</span>
                              <span className="ml-1.5 text-[11px] text-muted/60">{desc}</span>
                            </div>
                            <button
                              role="switch"
                              aria-checked={active}
                              aria-label={`Toggle ${label}`}
                              onClick={() =>
                                setEdit((p) =>
                                  p && { ...p, permissions: togglePerm(p.permissions, bit) }
                                )
                              }
                              className={clsx(
                                "relative h-4 w-7 shrink-0 rounded-full transition-colors",
                                active ? "bg-accent-600" : "bg-bg-600/60"
                              )}
                            >
                              <span
                                className={clsx(
                                  "absolute top-0.5 h-3 w-3 rounded-full bg-white shadow transition-transform",
                                  active ? "translate-x-3.5" : "translate-x-0.5"
                                )}
                              />
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Feedback + Save */}
            {saveError && <p className="text-xs text-red-400">{saveError}</p>}
            {saveOk && <p className="text-xs text-green-400">✓ Changes saved</p>}
            <button
              onClick={handleSave}
              disabled={saving || !edit.name.trim()}
              className="rounded-lg bg-accent-600 px-5 py-2 text-sm font-medium text-white hover:bg-accent-500 disabled:opacity-40 transition-colors"
            >
              {saving ? "Saving…" : "Save Changes"}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

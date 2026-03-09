import { useState } from "react";
import { useStore } from "../store";
import { invoke } from "../invoke";
import type { ImportJob, BulkInvitation } from "../store";

/**
 * ImportWizard — Data import from other platforms + bulk email invite.
 * Used in server settings or admin panel context.
 */
export default function ImportWizard({ serverId }: { serverId: string }) {
  const { importJobs, loadImportJobs } = useStore();
  const jobs = importJobs[serverId] ?? [];

  const [platform, setPlatform] = useState("discord");
  const [metadata, setMetadata] = useState("");
  const [creatingImport, setCreatingImport] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  // Bulk invite state
  const [emailsText, setEmailsText] = useState("");
  const [sendingInvites, setSendingInvites] = useState(false);
  const [inviteResult, setInviteResult] = useState<BulkInvitation | null>(null);
  const [inviteError, setInviteError] = useState<string | null>(null);

  const [activeTab, setActiveTab] = useState<"import" | "invite">("import");

  const handleCreateImport = async () => {
    setCreatingImport(true);
    setImportError(null);
    try {
      let parsed: unknown = null;
      if (metadata.trim()) {
        try {
          parsed = JSON.parse(metadata);
        } catch {
          setImportError("Metadata must be valid JSON");
          setCreatingImport(false);
          return;
        }
      }
      await invoke<ImportJob>("create_import_job", {
        serverId,
        sourcePlatform: platform,
        metadata: parsed,
      });
      await loadImportJobs(serverId);
    } catch (e: unknown) {
      setImportError(String(e));
    } finally {
      setCreatingImport(false);
    }
  };

  const handleBulkInvite = async () => {
    setSendingInvites(true);
    setInviteError(null);
    setInviteResult(null);
    try {
      const emails = emailsText
        .split(/[\n,;]+/)
        .map((e) => e.trim())
        .filter(Boolean);
      if (emails.length === 0) {
        setInviteError("Enter at least one email address");
        setSendingInvites(false);
        return;
      }
      const result = await invoke<BulkInvitation>("create_bulk_invitation", {
        serverId,
        emails,
      });
      setInviteResult(result);
      setEmailsText("");
    } catch (e: unknown) {
      setInviteError(String(e));
    } finally {
      setSendingInvites(false);
    }
  };

  const statusBadge = (status: string) => {
    const colors: Record<string, string> = {
      pending: "bg-yellow-500/20 text-yellow-300",
      processing: "bg-blue-500/20 text-blue-300",
      completed: "bg-green-500/20 text-green-300",
      failed: "bg-red-500/20 text-red-300",
      sending: "bg-blue-500/20 text-blue-300",
    };
    return (
      <span className={`text-xs px-2 py-0.5 rounded-full ${colors[status] ?? "bg-gray-500/20 text-gray-300"}`}>
        {status}
      </span>
    );
  };

  return (
    <div className="flex flex-col gap-4 p-4 max-w-2xl">
      <h2 className="text-lg font-bold text-white">Import & Migration</h2>

      {/* Tabs */}
      <div className="flex gap-2">
        <button
          onClick={() => setActiveTab("import")}
          className={`px-3 py-1.5 text-sm rounded ${activeTab === "import" ? "bg-indigo-600 text-white" : "bg-gray-700/50 text-gray-300 hover:bg-gray-700"}`}
        >
          Data Import
        </button>
        <button
          onClick={() => setActiveTab("invite")}
          className={`px-3 py-1.5 text-sm rounded ${activeTab === "invite" ? "bg-indigo-600 text-white" : "bg-gray-700/50 text-gray-300 hover:bg-gray-700"}`}
        >
          Bulk Invite
        </button>
      </div>

      {activeTab === "import" && (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-gray-400">
            Import messages, channels, and members from another platform.
          </p>

          <label className="text-sm text-gray-300">
            Source Platform
            <select
              value={platform}
              onChange={(e) => setPlatform(e.target.value)}
              className="mt-1 block w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
            >
              <option value="discord">Discord</option>
              <option value="slack">Slack</option>
              <option value="matrix">Matrix</option>
            </select>
          </label>

          <label className="text-sm text-gray-300">
            Metadata (JSON, optional)
            <textarea
              value={metadata}
              onChange={(e) => setMetadata(e.target.value)}
              rows={3}
              placeholder='{"export_path": "/path/to/export.zip"}'
              className="mt-1 block w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white font-mono"
            />
          </label>

          {importError && <p className="text-sm text-red-400">{importError}</p>}

          <button
            onClick={handleCreateImport}
            disabled={creatingImport}
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded text-sm w-fit"
          >
            {creatingImport ? "Starting…" : "Start Import"}
          </button>

          {/* Job list */}
          {jobs.length > 0 && (
            <div className="mt-4">
              <h3 className="text-sm font-semibold text-gray-300 mb-2">Import History</h3>
              <div className="space-y-2">
                {jobs.map((job) => (
                  <div key={job.id} className="bg-gray-800/50 rounded p-3 flex items-center justify-between">
                    <div>
                      <span className="text-sm text-white capitalize">{job.sourcePlatform}</span>
                      <span className="text-xs text-gray-500 ml-2">
                        {job.importedItems}/{job.totalItems} items
                      </span>
                    </div>
                    {statusBadge(job.status)}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === "invite" && (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-gray-400">
            Send email invitations to multiple people at once.
          </p>

          <label className="text-sm text-gray-300">
            Email Addresses (one per line, or comma-separated)
            <textarea
              value={emailsText}
              onChange={(e) => setEmailsText(e.target.value)}
              rows={5}
              placeholder={"alice@example.com\nbob@example.com"}
              className="mt-1 block w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-sm text-white"
            />
          </label>

          {inviteError && <p className="text-sm text-red-400">{inviteError}</p>}
          {inviteResult && (
            <p className="text-sm text-green-400">
              Invitation created! Code: {inviteResult.inviteCode} — {inviteResult.totalCount} recipients
            </p>
          )}

          <button
            onClick={handleBulkInvite}
            disabled={sendingInvites}
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded text-sm w-fit"
          >
            {sendingInvites ? "Sending…" : "Send Invitations"}
          </button>
        </div>
      )}
    </div>
  );
}

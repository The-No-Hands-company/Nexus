# Nexus Federation Guide

> **v0.8.5 — Federation UX**  
> This guide covers everything a community self-hoster needs to get their Nexus
> instance talking to the wider Nexus federation.

---

## Table of Contents

1. [What is Federation?](#what-is-federation)
2. [Quick Start (3 steps)](#quick-start)
3. [Setting Up Your Instance Identity](#instance-identity)
4. [Peering with Another Instance](#peering)
5. [Trust Levels Explained](#trust-levels)
6. [Managing Inbound Peer Requests](#inbound-requests)
7. [The `.well-known` Endpoint](#well-known)
8. [Blocking and Removing Peers](#blocking)
9. [The Federation Audit Log](#audit-log)
10. [Cross-Instance User Search](#user-search)
11. [Troubleshooting](#troubleshooting)
12. [Security Considerations](#security)

---

## What is Federation? {#what-is-federation}

Federation lets separate Nexus instances communicate so that users on one
instance can interact with users on another. Each instance remains independently
operated and owned — there is no central authority.

Key properties:

- **Opt-in** — you choose which instances to federate with.
- **Trust-scored** — you assign a trust score (0–100) to each peer.
- **Audit-logged** — every admin action is recorded.
- **No metadata leakage** — instances only share what they need to.

---

## Quick Start {#quick-start}

**Prerequisites:** You must hold the `INSTANCE_ADMIN` flag on your user account.
Ask your database admin to run:

```sql
UPDATE users SET flags = flags | (1 << 7) WHERE username = 'your_username';
```

**Step 1 — Set your instance identity** (Settings → Federation → Identity):

```
Display name:   My Community
Description:    A friendly community for hobbyists
Admin contact:  admin@myinstance.example
Policy:         Open
```

**Step 2 — Add a peer** (Settings → Federation → Peers → Add Peer):

```
Domain:        nexus.other.example
Trust score:   50
Message:       Hi! Let's federate.
```

Nexus will ping the remote instance's `.well-known` endpoint, store it, and
dispatch an outbound peering request.

**Step 3 — Wait for acceptance** (or accept an inbound request):

Once the remote admin accepts, users on both instances can interact.

---

## Setting Up Your Instance Identity {#instance-identity}

Your instance identity is published at `/.well-known/nexus/server` and is the
first thing remote admins see when evaluating a peering request.

| Field | Description | Example |
|-------|-------------|---------|
| **Display Name** | Human-readable instance name | `Hobby Hackers HQ` |
| **Description** | Short description (max 512 chars) | `A community for makers` |
| **Admin Contact** | Email or URI for federation queries | `admin@example.com` |
| **Federation Policy** | Who can peer with you | `open` / `closed` / `invite_only` |

### Federation Policies

| Policy | Meaning |
|--------|---------|
| `open` | Any remote instance can initiate peering. You still review requests. |
| `invite_only` | You must initiate all peering. Inbound requests are rejected automatically. |
| `closed` | Federation is administratively disabled. No new peering is accepted. |

---

## Peering with Another Instance {#peering}

### Initiating a Peer Request

Go to **Settings → Federation → Peers** and fill in the **Add Peer** form:

- **Domain**: the bare hostname of the remote instance (e.g. `nexus.example.org`).
  Do not include `https://` or paths.
- **Trust score**: initial trust (0–100). Default is 50. See [Trust Levels](#trust-levels).
- **Message**: optional text sent with the request (shown to the remote admin).

Nexus will immediately contact the remote instance to verify it's reachable. If
the ping fails, the request is not saved and you'll see an error.

### What Happens Next

1. An outbound `federation_peer_requests` row is created with `status = pending`.
2. The remote instance is notified (via its own `/federation/incoming-request` API).
3. When the remote admin accepts, both instances set `status = accepted` and
   users can interact across the boundary.
4. Until then, cross-instance features (shared channels, DMs, searches) are not
   available for that domain.

### Checking Peer Status

The **Peers** table shows real-time health:

| Indicator | Meaning |
|-----------|---------|
| `✓ 42ms` | Online, measured latency |
| `✗ offline` | Last ping failed |
| (stale) | Not pinged recently — click **Ping** to refresh |

Click **Ping** next to any peer to run a live health check and update the stored
latency.

---

## Trust Levels {#trust-levels}

Trust scores control features available to users from a remote instance.

| Score | Label | Meaning |
|-------|-------|---------|
| 80–100 | High | Full cross-instance features; users treated nearly like local users |
| 40–79 | Medium | Standard cross-instance features; default for new peers |
| 0–39 | Low | Restricted: read-only access, no DMs |
| Blocked | — | All traffic from this instance is rejected |

To edit a trust score, click the trust badge in the Peers table. An inline
number input appears — enter a value between 0 and 100 and click **Save**.

**Tip:** Start at 50 (medium) and raise to 80+ after building confidence in the
remote admin's moderation practices.

---

## Managing Inbound Peer Requests {#inbound-requests}

When another instance wants to peer with yours, a request appears in
**Settings → Federation → Requests** with a red badge counter.

For each inbound request you can see:

- Remote domain and display name
- Their federation policy and description
- The message they attached (if any)
- When the request was created

### Accepting

Click **Accept** to:

1. Mark the request as `accepted`.
2. Upsert the remote into `federated_servers` with a minimum trust score of 50.
3. Notify the remote instance (they'll see the status change).

### Rejecting

Click **Reject** to refuse the request. The remote admin is notified and can
re-apply in the future if you change your mind.

---

## The `.well-known` Endpoint {#well-known}

Nexus publishes instance metadata at:

```
GET https://your.domain/.well-known/nexus/server
```

Example response:

```json
{
  "server_name":        "your.domain",
  "software":          "nexus",
  "software_version":  "0.8.5",
  "display_name":      "My Community",
  "description":       "A friendly place for hobbyists",
  "admin_contact":     "admin@your.domain",
  "federation_policy": "open",
  "user_count":        142
}
```

### Nginx / Reverse Proxy

If you're behind a reverse proxy, make sure requests to `/.well-known/nexus/`
are forwarded to your Nexus API. Example Nginx snippet:

```nginx
location /.well-known/nexus/ {
    proxy_pass http://127.0.0.1:3000;
    proxy_set_header Host $host;
}
```

---

## Blocking and Removing Peers {#blocking}

### Blocking

Blocking a peer immediately stops all inbound traffic from that domain. The
peer remains in your database and you can unblock it later.

- **Block**: Settings → Federation → Peers → **Block** button.
- **Unblock**: Settings → Federation → Peers → **Unblock** button.

Blocked peers show in red in the trust column.

### Removing

Removing a peer deletes it from your `federated_servers` table entirely. Any
existing accepted peering requests are also cleaned up. This action cannot be
undone without re-initiating peering.

---

## The Federation Audit Log {#audit-log}

Every admin action is recorded:

| Action | Trigger |
|--------|---------|
| `peer_added` | Outbound peering request sent |
| `peer_removed` | Peer removed |
| `peer_blocked` | Peer blocked |
| `peer_unblocked` | Peer unblocked |
| `trust_updated` | Trust score changed |
| `request_accepted` | Inbound request accepted |
| `request_rejected` | Inbound request rejected |
| `identity_updated` | Instance identity fields saved |

You can filter the log by domain in **Settings → Federation → Audit Log**.

The log is append-only; entries are never deleted. Use it to prove compliance
or investigate incidents.

---

## Cross-Instance User Search {#user-search}

Authenticated users (not just admins) can search for users on federated
instances via the standard search UI. The backend queries
`GET /api/v1/federation/search?q=<query>&domain=<optional>`.

Results include: username, display name, avatar, and source instance domain.

**Privacy note:** The remote instance only receives the search query; it never
learns which local users are doing the searching.

---

## Troubleshooting {#troubleshooting}

### "Could not reach `nexus.example`: connection refused"

- Verify the remote instance is online and reachable from your server.
- Check that `/.well-known/nexus/server` returns 200 on the remote.
- Ensure no firewall blocks outbound HTTPS (port 443) from your instance.

### Peer shows `✗ offline` but the site is up

- Click **Ping** to force a fresh health check.
- The remote may have changed its domain — remove and re-add.
- Check the remote's TLS certificate for expiry.

### "Only inbound requests can be accepted" error

- You cannot accept your own outbound request — only the remote admin can.
- Check the **direction** column in the Requests tab.

### "Request is already accepted/rejected"

- The request was already acted on. Refresh the page to see the updated status.

### I can't see the Federation section in Settings

- Federation management requires the `INSTANCE_ADMIN` flag.
- Contact your database admin to grant it (see [Quick Start](#quick-start)).

### Federation is enabled but users can't interact cross-instance

- Check that the peering request status is `accepted` (not `pending`).
- Verify the trust score is ≥ 40 (medium tier).
- Ensure neither instance has the other blocked.

---

## Security Considerations {#security}

1. **Vet before peering.** Check the remote's `.well-known` page and reputation
   before accepting requests. A malicious instance can flood your users with
   spam.

2. **Start at medium trust (score 50).** Only raise to high (80+) after you've
   operated alongside the remote for a while and trust their moderation.

3. **Use `invite_only` for private instances.** If you're running a closed
   community, set your federation policy to `invite_only` so only you can
   initiate new peering.

4. **Monitor the audit log.** Check it regularly for unexpected actions. All
   admin operations are logged with timestamps.

5. **Block first, investigate later.** If a remote instance starts behaving
   badly (spam, harassment), block it immediately and investigate afterward.
   Blocking is instant; removal requires a confirm dialog.

6. **Admin contact.** Keep your `admin_contact` field up to date. Remote admins
   may need to reach you out-of-band about incidents.

---

*Last updated: v0.8.5 — Federation UX*

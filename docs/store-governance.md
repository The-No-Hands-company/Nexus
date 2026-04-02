# Nexus Store — Governance & Trust Model

## Vision

The Nexus Store enables creators to extend and customize Nexus while maintaining trust, safety, and compliance. The Store is not a marketplace for the company — it's infrastructure for the community. Nexus remains free; creators may charge for content if they pass vetting.

## Core Principles

1. **Privacy by Default**: TNHC never collects, stores, or can be compelled to hand over user identities.
   No government ID. No legal name. No phone number. No financial data. Not now, not ever.
2. **Platform Independence**: Nexus is neutral; Store policies serve all, favor none
3. **Creator Empowerment**: Low barrier to entry; clear path to monetization
4. **User Protection**: Rigorous vetting prevents malware, abuse, copyright violations
5. **Trust Tiers**: Users see creator identity level and review status
6. **Transparency**: Public audit trail; appeals process for rejections
7. **Phantom Protocol Alignment**: Governance must remain compatible with the upcoming
   Phantom Protocol design, including minimization of central observability and durable user metadata.

## What Can Be Sold

- **Plugins**: Extend Nexus functionality (bots, automation, integrations)
- **Themes**: Custom UI/UX styling and layouts
- **Templates**: Pre-configured server/workspace setups
- **Asset Packs**: Emojis, stickers, sounds, media libraries
- **Integrations**: Bridges to external services (GitHub, Slack, Discord, etc.)
- **Bots**: Automated assistants and workflows

## What Cannot Be Sold

- **Core Features**: Users shouldn't pay to unlock messaging, DMs, calls, etc.
- **Platform Lockin**: No plugins that create dependency on third-party platforms
- **Bypasses**: Circumventing Nexus policies (moderation, privacy, security)
- **Stolen Content**: Copyright infringement, trademark violations

## Vetting Pipeline

### Stage 1: Submission → Draft

Creator fills metadata:
- Name, description, category, icon
- Source code URL (public or private)
- Pricing (free or paid)
- Creator attestation (rights ownership, legal compliance)

### Stage 2: Initial Scan → Scanning

Automated checks:
- Malware signature scanning (ClamAV, Yara)
- Dependency vulnerability audit (SBOM analysis)
- Code pattern detection (suspicious APIs, backdoors)
- Manifest validation (permissions, resource limits)

**Result**: Flag for human review or auto-approve if clean

### Stage 3: Human Review → Review

Peer reviewers assess:
- Feature compatibility with Nexus APIs
- Policy compliance (no lockin, no bypasses)
- User privacy and data handling
- Code quality and stability

**Result**: Approved / Rejected / Request Changes

### Stage 4: Approval State

**Green States**:
- `approved` + `trust_tier=verified` — Creator identity cryptographically verified + code passed scanning
- `approved` + `trust_tier=reviewed` — Creator passed review; identity unverified
- `draft` — Private testing; not searchable

**Red States**:
- `rejected` — Policy violation; reason logged; can resubmit
- `quarantined` — Suspicious; quarantine reason logged; manual override needed
- `takedown` — Copyright/abuse reported; removed from marketplace; preserved for audit

## Trust Tiers

### Unlisted
- Status: Sandbox; not searchable
- Use: Private testing, internal builds
- Upgrade: Submit for review

### Reviewed
- Status: Publicly listed; passes human review
- Identity: Email verified minimum (creator account not compromised)
- Signals: Community ratings, download count, review history
- Display: "Human-reviewed" badge in Store

### Verified
- Status: Publicly listed; full cryptographic verification passed
- Requirements (any one of):
  - **Domain verification** — Creator proves ownership of a public domain via DNS TXT record
  - **Signature verification** — Creator controls a published PGP/SSH/minisign key and signs plugin manifests
  - **Third-party audit** — Open-source project with a published security audit
- Display: "Verified Creator" badge + verification method in Store
- Note: No personal identity is collected. Verification is cryptographic and/or publicly auditable.

## Creator Vetting

### Privacy-First Identity Model

TNHC does not ask who you are. We ask what you can prove.

Identity on Nexus is entirely **cryptographic and public** — you prove you control a key or a domain.
No documents are submitted. No personal data is stored. No government ID.
No outside authority can compel us to deanonymize creators because we never have that information.

### Identity Levels

1. **Unverified** (default)
   - Account created; basic functionality available
   - Risk: Account compromise possible

2. **Email Verified**
   - Email ownership confirmed via click-to-verify link
   - No personal data stored beyond the email address itself

3. **Domain Verified**
   - Creator proves ownership of a domain via DNS TXT record challenge
   - TNHC records only the domain string (public information)
   - Suitable for: organizations, projects, teams
   - Challenge: `nexus-verify=<token>` must be present in DNS TXT on `_nexus.<domain>`

4. **Signature Verified**
   - Creator controls a public cryptographic key (PGP, SSH, or minisign)
   - Key must be published at a public, independently-verifiable location
     (e.g., `keys.openpgp.org`, GitHub profile, personal website)
   - TNHC stores only the **public key fingerprint** — not the key, not the identity behind it
   - Plugin manifests are signed by this key; users can independently verify without trusting TNHC
   - This is the highest identity tier. It proves authorship without revealing identity.

### Vetting Requirements by Tier

| Tier | Identity Level | Scan Status | Reviewer Approval | Monetization |
|------|---|---|---|---|
| Unlisted | Unverified | Any | Not required | Not allowed |
| Reviewed | Email Verified | Clean | Required | Allowed |
| Verified | Domain or Signature Verified | Clean | Required | Allowed |

### Vetting Decision Tree

```
Creator submits plugin
   ↓
[Automated Scan]
   ├─ CLEAN → Human Review (reviewed tier candidate)
   ├─ SUSPICIOUS → Quarantine + manual override required
   └─ MALWARE → Reject + block creator account
   ↓
[Human Reviewer Assessment]
   ├─ Policy violation → Rejected (resubmit with changes)
   ├─ Marginal quality → Request changes (feedback + resubmit)
   └─ Approved → Listed (unlisted → reviewed → verified as creator proves trustworthiness)
```

## Monetization Model

### Payments-Neutral Architecture

TNHC does not process payments. This is not a cost-cutting decision — it is a **core privacy commitment**.

If TNHC collected transaction data, payout addresses, or financial records, that data could be
subpoenaed, hacked, leaked, or otherwise used against creators. We refuse to hold it.

### How Paid Plugins Work

1. Creator sets a price and provides their **own payment link** (Stripe Checkout, Ko-fi, Gumroad, etc.)
2. When a user wants to install a paid plugin, Nexus redirects them to the creator's payment link
3. The creator's payment processor handles the transaction entirely
4. Creator sets up their own fulfillment (e.g., delivery of a license key or access token)
5. TNHC tracks **install count** only — not transaction amounts, not payment methods, not identities

### What TNHC Stores

| Data | Stored? | Notes |
|------|---------|-------|
| Plugin price (for display) | ✅ Yes | Public information; shown in Store listing |
| Payment link URL | ✅ Yes | Creator-provided; publicly shown |
| Install/purchase count | ✅ Yes | Non-financial counter only |
| Transaction amounts | ❌ Never | Not our data to hold |
| Payout addresses | ❌ Never | Creator manages their own payment accounts |
| Creator earnings | ❌ Never | Cannot be compelled to reveal |
| Buyer identity | ❌ Never | Handled entirely by creator's payment processor |

### Creator Responsibility

- All tax compliance, consumer protection, refunds, and chargebacks are the creator's responsibility
- TNHC provides no payment infrastructure and has no liability for creator transactions
- Creators choose their payment processor; they control their own financial data

## Quarantine & Takedown

### Quarantine (Temporary Suspension)
- **Trigger**: Suspicious pattern detected or community report
- **Effect**: Removed from search; existing installs continue but no new installs
- **Duration**: 24–72 hours pending manual review
- **Outcome**: Restore, reject, or escalate to takedown

### Takedown (Permanent Removal)
- **Trigger**: Copyright claim, malware confirmed, or severe policy violation
- **Effect**: Removed from search; existing installs notified to uninstall
- **Duration**: Permanent unless appealed and overturned
- **Audit**: Preserved in DB with reason and reporter identity (if reporter was logged in)
- **Appeal**: Creator can file dispute; reviewed by second reviewer

### Takedown Reasons
- `copyright` — Intellectual property infringement
- `malware` — Confirmed security threat
- `abuse` — Harassment, hate speech, or targeted abuse
- `spam` — Deceptive or low-quality content
- `tos_violation` — Violates Nexus terms of service

### Appeals Process
1. Creator files dispute with evidence
2. Senior reviewer assesses (48–72 hours)
3. Determination: Overturn or uphold
4. If overturned: Reinstated to marketplace; reviewed tier

## Module-Level Access Control

Plugins can declare required or optional Nexus modules:
- **Required**: Plugin doesn't function without module (e.g., voice bot requires `voice_channels`)
- **Optional**: Plugin works degraded without module (e.g., automation bot works without `federation`)

API returns `effective_enabled_modules` per user; plugin installer validates compatibility:
```
IF plugin.required_modules ⊆ user.effective_enabled_modules
  THEN install allowed
  ELSE show error "This plugin requires X, Y, Z modules not available in your account"
```

Clients enforce this via UI (hide install button if incompatible).

## Admin Dashboard

### Review Queue
- Filters: Submitted, Scanning, In Review
- Sort: Created date (oldest first)
- Bulk: Approve multiple, reject with reason, quarantine
- Audit: All decisions logged with reviewer ID + timestamp

### Creator Vetting
- Search: Creator domain, signing key fingerprint, or user ID
- Status: Pending, Approved, Rejected, Suspended
- Action: Approve identity tier, suspend account
- Note: Admins never see personal documents because none are collected

### Takedowns
- Status: Pending, Quarantined, Reviewed, Reinstated, Permanent Takedown
- Report View: Reason, evidence URLs, reporter identity (if logged-in)
- Action: Quarantine, approve takedown, reinstate, override

### Store Stats
- Per-plugin: Install count, review status, trust tier
- Per-creator: Identity level, vetting status, active plugins
- No financial data — TNHC has none

## API Endpoints

### Creator Publishing
```
POST /marketplace/plugins                         # Submit for review
POST /marketplace/plugins/:plugin_id/submit-review # Request review after edits
```

### Admin Review
```
GET  /marketplace/admin/review-queue              # List pending review
POST /marketplace/admin/plugins/:id/approve       # Approve + set tier
POST /marketplace/admin/plugins/:id/reject        # Reject with reason
POST /marketplace/admin/plugins/:id/quarantine    # Quarantine as suspicious
POST /marketplace/admin/plugins/:id/takedown      # Request takedown (security)
POST /marketplace/admin/takedowns/:id/review      # Review takedown report
POST /marketplace/admin/takedowns/:id/reinstate   # Reinstate plugin
```

### Creator Vetting
```
GET  /marketplace/creator/vetting                 # Get own vetting record
POST /marketplace/creator/vetting                 # Apply for vetting
GET  /marketplace/admin/creators/vetting-queue    # Admin: list pending vetting
POST /marketplace/admin/creators/:id/approve      # Admin: approve identity tier
POST /marketplace/admin/creators/:id/reject       # Admin: reject vetting application
```

### User Discovery
```
GET /marketplace/plugins                          # Search + filter by tier
GET /marketplace/plugins/:slug                    # Get plugin details (includes trust tier)
```

## Security & Privacy

### What TNHC Cannot Hand Over

If TNHC receives a subpoena, law enforcement request, or any compelled disclosure demand,
we cannot hand over what we do not have:

- ❌ Creator legal names or government IDs — never collected
- ❌ Financial records or payout destinations — never stored
- ❌ Identity documents — never collected
- ❌ IP address history — not retained
- ❌ Communication content — E2EE where applicable; servers see ciphertext only

### Phantom Protocol Compatibility (Planned)

Nexus Store governance is being built to compose cleanly with Phantom Protocol.

- Verification remains cryptographic (domain + key proofs), not identity-document based
- Moderation workflows operate on plugin artifacts and signed metadata, not user dossiers
- Future protocol rollouts must preserve zero-retention guarantees already defined here

### What Is Stored

| Data | Stored | Purpose |
|------|--------|---------|
| User ID (UUID) | ✅ | Session linking only |
| Email address (optional) | ✅ | Account recovery only; user can omit |
| Domain (if domain-verified) | ✅ | Public; DNS is already public |
| Public key fingerprint (if sig-verified) | ✅ | Public; key itself is published by creator |
| Plugin metadata | ✅ | Store listing |
| Review audit log | ✅ | Accountability for admin actions |
| Install/purchase counts | ✅ | Non-identifying aggregate |

### Signing & Manifest Verification

- Plugins may include a `signature` field in their manifest
- Signature is verified against the creator's registered public key fingerprint
- Users can independently verify signatures without trusting TNHC
- Unsigned plugins are listed but shown without the "Verified" badge

### IP Whitelisting (Optional, Creator-Opted)
- Creator can restrict plugin publishing to specific IP ranges
- This is a creator-controlled security feature; a creator-provided list
- Prevents account takeover from unknown locations

### Logging & Audit Trail
- All review decisions logged with timestamp + reviewer ID
- No personal data in audit logs beyond internal UUID references
- No financial transaction logs (TNHC has none to log)

## FAQ

**Q: Do I need to provide ID to publish a plugin?**
A: No. Never. TNHC will not ask for or accept government ID, legal names, or any personal documents.
The highest identity tier is cryptographic key verification — you prove you control a key, nothing more.

**Q: Can I charge for my plugin?**
A: Yes, if you pass review. You set your own price and provide your own payment link (Stripe, Ko-fi, etc.).
TNHC redirects buyers to your link; we don't handle the transaction.

**Q: Does TNHC take a cut?**
A: No. TNHC does not process payments, therefore there is no cut to take.
You keep 100% of what your payment processor sends you.

**Q: What if my plugin is rejected?**
A: You'll receive a reason and can resubmit after addressing feedback.

**Q: Can Nexus sell plugins?**
A: No. The company doesn't create Store content. All content is by creators.

**Q: What about copyright?**
A: You attest you own or have rights to all content. Nexus honors takedown requests per DMCA.

**Q: How do I report a malicious plugin?**
A: Use the "Report" button in the Store or email security@nexus.app with details.

**Q: Can I contest a takedown?**
A: Yes, file an appeal with evidence. We review within 72 hours.

**Q: Can TNHC be compelled to identify a creator?**
A: We structurally cannot. Creator identity is never collected. We hold only a public key fingerprint
or a domain string — both of which were already public before you told us about them.

---

**Policy Version**: 2.0 (April 2026)
**Status**: Live
**Last Updated**: April 2, 2026
**Maintained By**: The No Hands Company (Community Team)


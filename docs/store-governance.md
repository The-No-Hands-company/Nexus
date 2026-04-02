# Nexus Store — Governance & Trust Model

## Vision

The Nexus Store enables creators to extend and customize Nexus while maintaining trust, safety, and compliance. The Store is not a marketplace for the company — it's infrastructure for the community. Nexus remains free; creators may charge for content if they pass vetting.

## Core Principles

1. **Platform Independence**: Nexus is neutral; Store policies serve all, favor none
2. **Creator Empowerment**: Low barrier to entry; clear path to monetization
3. **User Protection**: Rigorous vetting prevents malware, abuse, copyright violations
4. **Trust Tiers**: Users see creator identity level and review status
5. **Transparency**: Public audit trail; appeals process for rejections

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
- `approved` + `trust_tier=verified` — Creator identity verified + code passed scanning
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
- Status: Publicly listed; full verification passed
- Identity: Legal entity verified OR significant security audit passed
- Requirements:
  - Domain ownership verification (corporate plugins)
  - OR legal identity verification (individual creators)
  - OR third-party security audit (open-source projects)
- Display: "Verified Creator" badge + identity proof in Store

## Creator Vetting

### Identity Levels

1. **Unverified** (default)
   - Email ownership confirmed
   - Risk: Account compromise possible

2. **Email Verified**
   - Email ownership confirmed
   - Backup recovery email

3. **Domain Verified**
   - Organization domain DNS verification
   - Risk: Internal employee account creation

4. **Legal Verified**
   - Legal entity / individual identity verified
   - Government ID or business registration
   - Risk: Minimal; highest trust

### Vetting Requirements by Tier

| Tier | Identity Level | Scan Status | Reviewer Approval | Monetization |
|------|---|---|---|---|
| Unlisted | Unverified | Any | Not required | Not allowed |
| Reviewed | Email Verified | Clean | Required | Allowed (70/30) |
| Verified | Domain/Legal | Clean | Required | Allowed (70/30) |

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

### Pricing
- **Free**: Zero cost; listed immediately after review
- **Paid**: Creator sets price in USD (other currencies via conversion)
- **Freemium**: Free base + paid upgrades (handled via metadata)

### Revenue Split
- Creator: 70%
- Nexus Platform: 30%
- Split applied post-transaction; payouts monthly

### Compliance
- Creator responsible for GTM compliance (taxes, consumer protection, refunds)
- Nexus provides payment rails (Stripe integration); no liability for creator taxes
- Refund policy: User-initiated chargebacks default to Stripe handling

### Payout
- Minimum threshold: $10 USD
- Schedule: Monthly, 5th business day
- Method: Direct bank transfer, PayPal, or equivalent
- Encryption: Payout addresses stored encrypted in DB; decryption restricted to finance system

## Quarantine & Takedown

### Quarantine (Temporary Suspension)
- **Trigger**: Suspicious pattern detected or customer report
- **Effect**: Removed from search; existing installs continue but no new installs
- **Duration**: 24–72 hours pending manual review
- **Outcome**: Restore, reject, or escalate to takedown

### Takedown (Permanent Removal)
- **Trigger**: Copyright claim, malware confirmed, or severe policy violation
- **Effect**: Removed from search; existing installs notified to uninstall
- **Duration**: Permanent unless appealed and overturned
- **Audit**: Preserved in DB with reason and reporter identity
- **Appeal**: Creator can file dispute; reviewed by second reviewer + legal if needed

### Takedown Reasons
- `copyright` — Intellectual property infringement
- `malware` — Confirmed security threat
- `abuse` — Harassment, hate speech, or targeted abuse
- `spam` — Deceptive or low-quality content
- `tos_violation` — Violates Nexus terms of service

### Appeals Process
1. Creator files dispute with evidence
2. Senior reviewer + legal review (48-72 hours)
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
- Search: Creator email, domain, or user ID
- Status: Pending, Approved, Rejected, Suspended
- Action: Approve identity, request docs, suspend account

### Takedowns
- Status: Pending, Quarantined, Reviewed, Reinstated, Permanent Takedown
- Report View: Reason, evidence URLs, reporter identity (if logged-in)
- Action: Quarantine, approve takedown, reinstate, override

### Monetization Ledger
- Per-plugin: Revenue, creator earnings, platform earnings
- Per-creator: Lifetime sales, pending payout, payout history
- Export: CSV for accounting

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

### User Discovery
```
GET /marketplace/plugins                          # Search + filter by tier
GET /marketplace/plugins/:slug                    # Get plugin details (includes trust tier)
```

## Security & Privacy

### Data Encryption
- Identity documents: Encrypted at rest (AES-256 GCM)
- Payout addresses: Encrypted at rest
- Legal entity ID: Encrypted at rest
- Decryption: Restricted to service accounts (finance, vetting system)

### IP Whitelisting (Optional)
- Creator can whitelist IPs for plugin publishing
- Prevents account takeover via brute force
- Requires two-factor authentication

### Logging & Audit Trail
- All review decisions logged with timestamp + reviewer ID
- All monetization transactions logged
- All identity verifications logged
- Retention: 7 years (compliance)

## FAQ

**Q: Can I charge for my plugin?**
A: Yes, if you pass review and your creator account is verified.

**Q: What if my plugin is rejected?**
A: You'll receive a reason and can resubmit after addressing feedback.

**Q: Can Nexus sell plugins?**
A: No. The company doesn't create Store content. All content is by creators.

**Q: What about copyright?**
A: You confirm you own or have rights to all content. Nexus doesn't moderate copyright; we honor takedown requests per DMCA.

**Q: How do I report a malicious plugin?**
A: Use the "Report" button in the Store or email security@nexus.app with details.

**Q: Can I contest a takedown?**
A: Yes, file an appeal with evidence. We review within 72 hours.

---

**Policy Version**: 1.0 (February 2026)  
**Status**: Live  
**Last Updated**: February 4, 2026  
**Maintained By**: The No Hands Company (Community Team)

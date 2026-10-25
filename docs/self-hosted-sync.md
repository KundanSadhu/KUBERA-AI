# Self-hosted multi-user sync — selected pilot architecture

## Decisions captured

The selected first deployment shape is a **self-hosted KUBERA sync service** with **KUBERA-managed local accounts** and a **SQLite server database**.

This is a single-host pilot design, not a production-scale multi-user deployment. SQLite can provide transactional persistence for a small single-node service but does not replace a managed HA database, backup regime, or independent security review.

## Required security boundary

Before a client sync implementation is enabled, the service must provide all of the following:

1. TLS termination at a controlled reverse proxy; no plaintext remote sync endpoint.
2. Argon2id password hashes with per-user salts; passwords never leave the service as stored/retrievable data.
3. Opaque, hashed, revocable sessions with expiry, rotation, device metadata, and CSRF protection for browser surfaces.
4. Workspace membership and roles (`owner`, `operator`, `reviewer`) checked server-side on every read/write.
5. Immutable audit events for membership changes, login/session actions, record mutations, artifact publication, and sync conflicts.
6. Per-workspace object storage keys and content hashes; artifact bytes never become public URLs.
7. Optimistic concurrency/version checks and an explicit conflict record rather than silent last-write-wins for business records.
8. Encrypted backups and restore drills.
9. Rate limiting and account lockout/step-up controls for failed logins.

## Synchronization contract

Desktop clients will sync typed, append-only change records rather than accepting arbitrary SQL or filesystem changes. Each mutation includes a workspace ID, actor ID, object type, object ID, version, operation type, payload hash, and timestamp. The service validates authorization and version, commits the change and audit event atomically, and returns a cursor-based change feed.

Office deliverable bytes remain local until a human-approved publication action submits a bounded attachment under the workspace policy. Model prompts, OpenCode credentials, SMTP credentials, Slack tokens, raw sidecar events, and unapproved local files are never sync objects.

## Pilot acceptance gates

- A deployed TLS reverse proxy and a configured service base URL.
- An administrator bootstrap and secure recovery process.
- A tested SQLite backup/restore procedure for the single host.
- Independent threat-model and authentication review before any customer or multi-tenant data is synchronized.

Until these gates are met, KUBERA remains local-first and does not claim secure remote multi-user operation.

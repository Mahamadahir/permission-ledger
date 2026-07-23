# Security

Release 1 uses Argon2id password hashing, HTTP-only cookie sessions, CSRF tokens for cookie-authenticated mutations, and hashed extension tokens.

Audit logs record authentication, consent record changes, exports, and extension pairing or revocation. They intentionally avoid passwords, raw tokens, and request bodies.

User isolation is enforced in every consent record, export, extension device, and dashboard query by filtering on the authenticated `user_id`.

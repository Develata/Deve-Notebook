<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# security

## Purpose

Cryptographic primitives and security infrastructure: identity keypairs (Ed25519), content cipher (AES-GCM), password hashing (Argon2), key storage, and E2E encryption support.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Module entry, IdentityKeyPair type |
| `keypair.rs` | Ed25519 keypair generation and management |
| `cipher.rs` | AES-GCM content encryption/decryption |
| `hashing.rs` | Argon2 password hashing |
| `storage.rs` | Key storage on filesystem |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `auth/` | Authentication (JWT, passwords, config) |
| `permission/` | Permission system |

## For AI Agents

### Working In This Directory

- See `09_auth.md` in deve-note plan for auth/encryption design.
- `cipher.rs` handles E2E encryption of document content.
- Never log or expose key material.

<!-- MANUAL: -->

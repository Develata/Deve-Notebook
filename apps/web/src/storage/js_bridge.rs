//! plan_ref:
//!   - 04_storage#browser-storage-layering

use js_sys::{Promise, Uint8Array};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
const DB_NAME = 'deve_weblightpeer';
const DB_VERSION = 1;
const IDENTITY = 'peer_identity';
const META = 'repo_meta';
const CACHE = 'offline_cache';
const req = (r) => new Promise((ok, err) => { r.onsuccess = () => ok(r.result); r.onerror = () => err(r.error || new Error('IndexedDB request failed')); });
const openDb = () => new Promise((ok, err) => {
  if (!globalThis.indexedDB) return err(new Error('IndexedDB unavailable'));
  const r = globalThis.indexedDB.open(DB_NAME, DB_VERSION);
  r.onupgradeneeded = () => {
    const db = r.result;
    if (!db.objectStoreNames.contains(IDENTITY)) db.createObjectStore(IDENTITY, { keyPath: 'repoId' });
    if (!db.objectStoreNames.contains(META)) db.createObjectStore(META, { keyPath: 'repoId' });
    if (!db.objectStoreNames.contains(CACHE)) db.createObjectStore(CACHE, { keyPath: 'cacheKey' });
  };
  r.onsuccess = () => ok(r.result);
  r.onerror = () => err(r.error || new Error('IndexedDB open failed'));
});
const sha256Hex = async (bytes) => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).map((b) => b.toString(16).padStart(2, '0')).join('');
async function generateIdentity(repoId) {
  const pair = await crypto.subtle.generateKey({ name: 'Ed25519' }, false, ['sign', 'verify']);
  const publicKey = new Uint8Array(await crypto.subtle.exportKey('raw', pair.publicKey));
  return { repoId, peerId: (await sha256Hex(publicKey)).slice(0, 12), publicKey: Array.from(publicKey), privateKey: pair.privateKey, createdAt: Date.now() };
}
export async function probeStorageCapabilities() {
  const caps = { webcrypto: !!globalThis.crypto?.subtle, indexed_db: false, local_storage: false, ed25519: false };
  try { globalThis.localStorage.setItem('__deve_probe__', '1'); globalThis.localStorage.removeItem('__deve_probe__'); caps.local_storage = true; } catch {}
  try { const db = await openDb(); caps.indexed_db = true; db.close(); } catch {}
  if (caps.webcrypto) { try { const pair = await crypto.subtle.generateKey({ name: 'Ed25519' }, false, ['sign', 'verify']); caps.ed25519 = !!pair?.privateKey; } catch {} }
  return JSON.stringify(caps);
}
export async function loadOrCreateIdentity(repoId) {
  if (!globalThis.crypto?.subtle) throw new Error('WebCrypto unavailable');
  const db = await openDb();
  try {
    const store = db.transaction(IDENTITY, 'readwrite').objectStore(IDENTITY);
    let record = await req(store.get(repoId));
    if (!record?.privateKey || !record?.publicKey?.length) { record = await generateIdentity(repoId); await req(store.put(record)); }
    return JSON.stringify({ repo_id: record.repoId, peer_id: record.peerId, public_key: Array.from(record.publicKey), created_at: record.createdAt });
  } finally { db.close(); }
}
export async function signPeerMessage(repoId, bytes) {
  const db = await openDb();
  try {
    const record = await req(db.transaction(IDENTITY, 'readonly').objectStore(IDENTITY).get(repoId));
    if (!record?.privateKey) throw new Error('Missing stored private key');
    return new Uint8Array(await crypto.subtle.sign({ name: 'Ed25519' }, record.privateKey, bytes));
  } finally { db.close(); }
}
export async function loadRepoMeta(repoId) {
  const db = await openDb();
  try {
    const record = (await req(db.transaction(META, 'readonly').objectStore(META).get(repoId))) || { repoId, vector_json: null, last_handshake_ms: null };
    return JSON.stringify({ repo_id: record.repoId ?? repoId, vector_json: record.vector_json ?? null, last_handshake_ms: record.last_handshake_ms ?? null });
  } finally { db.close(); }
}
export async function mergeRepoMeta(repoId, patchJson) {
  const db = await openDb();
  try {
    const store = db.transaction(META, 'readwrite').objectStore(META);
    const current = (await req(store.get(repoId))) || { repoId, vector_json: null, last_handshake_ms: null };
    const patch = JSON.parse(patchJson);
    await req(store.put({ ...current, repoId, vector_json: patch.vector_json ?? current.vector_json ?? null, last_handshake_ms: patch.last_handshake_ms ?? current.last_handshake_ms ?? null }));
  } finally { db.close(); }
}
export async function touchOfflineCache(repoId, cacheKey) {
  const db = await openDb();
  try { await req(db.transaction(CACHE, 'readwrite').objectStore(CACHE).put({ cacheKey: `${repoId}::${cacheKey}`, repoId, updatedAt: Date.now() })); }
  finally { db.close(); }
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = probeStorageCapabilities)]
    pub(super) fn probe_storage_capabilities() -> Promise;
    #[wasm_bindgen(js_name = loadOrCreateIdentity)]
    pub(super) fn load_or_create_identity(repo_id: &str) -> Promise;
    #[wasm_bindgen(js_name = signPeerMessage)]
    pub(super) fn sign_peer_message(repo_id: &str, bytes: &Uint8Array) -> Promise;
    #[wasm_bindgen(js_name = loadRepoMeta)]
    pub(super) fn load_repo_meta(repo_id: &str) -> Promise;
    #[wasm_bindgen(js_name = mergeRepoMeta)]
    pub(super) fn merge_repo_meta(repo_id: &str, patch_json: &str) -> Promise;
    #[wasm_bindgen(js_name = touchOfflineCache)]
    pub(super) fn touch_offline_cache(repo_id: &str, cache_key: &str) -> Promise;
}

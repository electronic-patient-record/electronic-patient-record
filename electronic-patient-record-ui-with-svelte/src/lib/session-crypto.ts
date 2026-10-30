// Encryption for session records kept outside this process (OS-8). A shared store holds access and
// refresh tokens, which must never sit in a shared service in the clear: each record is sealed with
// AES-256-GCM under a key only the UI replicas hold, and bound to its storage key so a record
// copied under another key does not open.
import {
  createCipheriv,
  createDecipheriv,
  createHash,
  randomBytes,
} from "node:crypto";

const KEY_BYTES = 32;
const IV_BYTES = 12;
const TAG_BYTES = 16;

/** Parses a base64 or base64url key; throws unless it is exactly 32 bytes. */
export function parseKey(value: string): Buffer {
  const key = Buffer.from(value.trim(), "base64url");
  if (key.length !== KEY_BYTES) {
    throw new Error(
      `EPR_SESSION_ENCRYPTION_KEY must be ${KEY_BYTES} bytes, base64 or base64url encoded (got ${key.length})`,
    );
  }
  return key;
}

/** `iv.tag.ciphertext`, each base64url. `aad` is bound but not stored. */
export function seal(key: Buffer, aad: string, plaintext: string): string {
  const iv = randomBytes(IV_BYTES);
  const cipher = createCipheriv("aes-256-gcm", key, iv);
  cipher.setAAD(Buffer.from(aad));
  const body = Buffer.concat([
    cipher.update(plaintext, "utf8"),
    cipher.final(),
  ]);
  return [iv, cipher.getAuthTag(), body]
    .map((b) => b.toString("base64url"))
    .join(".");
}

/** The plaintext, or null when the record is malformed, tampered with, bound to another key or sealed with another key. */
export function open(key: Buffer, aad: string, sealed: string): string | null {
  const parts = sealed.split(".");
  if (parts.length !== 3) return null;
  try {
    const [ivText, tagText, bodyText] = parts;
    if (ivText === undefined || tagText === undefined || bodyText === undefined)
      return null;
    const iv = Buffer.from(ivText, "base64url");
    const tag = Buffer.from(tagText, "base64url");
    const body = Buffer.from(bodyText, "base64url");
    if (iv.length !== IV_BYTES || tag.length !== TAG_BYTES) return null;
    const decipher = createDecipheriv("aes-256-gcm", key, iv);
    decipher.setAAD(Buffer.from(aad));
    decipher.setAuthTag(tag);
    return Buffer.concat([decipher.update(body), decipher.final()]).toString(
      "utf8",
    );
  } catch {
    return null;
  }
}

/** The storage key for a secret id: a hash, so the store never holds a value that works as a cookie. */
export function storageKey(prefix: string, id: string): string {
  return `${prefix}${createHash("sha256").update(id).digest("hex")}`;
}

// A key for one create (OF-1). The API runs a create sent with the same key and the same body once, so a
// double click or a retry after a timeout cannot store a record twice. A form makes a key when it opens and
// a new one after each success; a refused request releases its key, so a corrected form can reuse it.

export function newIdempotencyKey(): string {
  return crypto.randomUUID();
}

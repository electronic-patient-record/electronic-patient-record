import { noteActivity } from "./idle.js";

// One place the pages call the API through the backend-for-frontend proxy. A network failure is
// status 0, so a page handles "could not reach the server" the same way as any other refusal.

export interface ApiResult<T = unknown> {
  status: number;
  body: T;
}

// The purpose of use sent when a call names none: the staff member's current choice (see
// `purpose.ts`). A call that passes its own purpose, such as "my record", is not affected.
let defaultPurpose: string | undefined;

export function setDefaultPurpose(purpose: string | undefined): void {
  defaultPurpose = purpose;
}

export async function call<T = unknown>(
  method: "GET" | "POST" | "PUT",
  path: string,
  body?: unknown,
  purpose?: string,
  // A background look does not keep the session alive (CQ-8).
  { background = false }: { background?: boolean } = {},
): Promise<ApiResult<T>> {
  if (!background) noteActivity();
  try {
    const res = await fetch(`/api/proxy${path}`, {
      method,
      headers: {
        ...(background ? { "x-epr-background": "1" } : {}),
        ...(body === undefined ? {} : { "content-type": "application/json" }),
        ...((purpose ?? defaultPurpose)
          ? { "x-purpose-of-use": (purpose ?? defaultPurpose)! }
          : {}),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await res.text();
    let parsed: unknown = null;
    try {
      parsed = text ? JSON.parse(text) : null;
    } catch {
      parsed = null;
    }
    return { status: res.status, body: parsed as T };
  } catch {
    return { status: 0, body: null as T };
  }
}

/** Fetch a binary answer (a photo) as a Blob, with the same purpose-of-use rule as `call`. */
export async function fetchBlob(
  path: string,
  purpose?: string,
): Promise<ApiResult<Blob | null>> {
  try {
    const p = purpose ?? defaultPurpose;
    const res = await fetch(`/api/proxy${path}`, {
      headers: p ? { "x-purpose-of-use": p } : {},
    });
    return { status: res.status, body: res.ok ? await res.blob() : null };
  } catch {
    return { status: 0, body: null };
  }
}

/** Send raw bytes (a photo) with PUT or POST; the answer is JSON like `call`. */
export async function sendBytes<T = unknown>(
  method: "PUT" | "POST",
  path: string,
  bytes: Blob | null,
  purpose?: string,
): Promise<ApiResult<T>> {
  try {
    const p = purpose ?? defaultPurpose;
    const res = await fetch(`/api/proxy${path}`, {
      method,
      headers: {
        "content-type": bytes?.type || "application/octet-stream",
        ...(p ? { "x-purpose-of-use": p } : {}),
      },
      body: bytes ?? undefined,
    });
    const text = await res.text();
    let parsed: unknown = null;
    try {
      parsed = text ? JSON.parse(text) : null;
    } catch {
      parsed = null;
    }
    return { status: res.status, body: parsed as T };
  } catch {
    return { status: 0, body: null as T };
  }
}

/** The message key for the statuses every page treats alike, or null for the page to handle. */
export function commonProblem(
  status: number,
): "signInRequired" | "forbidden" | "error" | null {
  if (status === 401) return "signInRequired";
  if (status === 403) return "forbidden";
  if (status === 0 || status >= 500) return "error";
  return null;
}

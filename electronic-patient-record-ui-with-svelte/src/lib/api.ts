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
): Promise<ApiResult<T>> {
  try {
    const res = await fetch(`/api/proxy${path}`, {
      method,
      headers: {
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

/** The message key for the statuses every page treats alike, or null for the page to handle. */
export function commonProblem(
  status: number,
): "signInRequired" | "forbidden" | "error" | null {
  if (status === 401) return "signInRequired";
  if (status === 403) return "forbidden";
  if (status === 0 || status >= 500) return "error";
  return null;
}

import { describe, expect, it } from "vitest";
import {
  DEFAULT_PURPOSE,
  STORAGE_KEY,
  parseStaffPurpose,
  readStoredPurpose,
  storePurpose,
} from "../../src/lib/purpose";
import { call, setDefaultPurpose } from "../../src/lib/api";
import { afterEach, vi } from "vitest";

afterEach(() => {
  setDefaultPurpose(undefined);
  vi.unstubAllGlobals();
});

describe("staff purposes", () => {
  it("offers treatment and operations, and nothing that implies an override", () => {
    expect(parseStaffPurpose("treatment")).toBe("treatment");
    expect(parseStaffPurpose("operations")).toBe("operations");
    for (const other of [
      "emergency",
      "research",
      "audit",
      "payment",
      "patient-access",
      "",
      null,
      3,
    ]) {
      expect(parseStaffPurpose(other)).toBeNull();
    }
  });

  it("remembers the choice, ignores junk, and survives blocked storage", () => {
    const store = new Map<string, string>();
    const storage = {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
    };
    expect(readStoredPurpose(storage)).toBe(DEFAULT_PURPOSE);
    storePurpose(storage, "treatment");
    expect(readStoredPurpose(storage)).toBe("treatment");
    store.set(STORAGE_KEY, "research");
    expect(readStoredPurpose(storage)).toBe(DEFAULT_PURPOSE);
    const blocked = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    };
    expect(readStoredPurpose(blocked)).toBe(DEFAULT_PURPOSE);
    expect(() => storePurpose(blocked, "treatment")).not.toThrow();
    expect(readStoredPurpose(undefined)).toBe(DEFAULT_PURPOSE);
  });
});

describe("call and the declared purpose", () => {
  const headersOf = async (purpose?: string) => {
    const fetchMock = vi.fn(async () => new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    await call("GET", "/x", undefined, purpose);
    return (fetchMock.mock.calls[0] as unknown as [string, RequestInit])[1]
      .headers;
  };
  it("uses the declared purpose unless the call names its own", async () => {
    expect(await headersOf()).toEqual({});
    setDefaultPurpose("treatment");
    expect(await headersOf()).toEqual({ "x-purpose-of-use": "treatment" });
    expect(await headersOf("patient-access")).toEqual({
      "x-purpose-of-use": "patient-access",
    });
  });
});

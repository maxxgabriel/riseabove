// One transport for everything: `call(method, args)`. In the desktop shell it is a
// Tauri command; in a browser it is a POST to the development server.

import type { ErrorKind } from "./contract.generated";

export class ApiError extends Error {
  code: string;
  /** The category of the failure: "you may not know this" is not "the program failed". */
  kind: ErrorKind | null;
  retryable: boolean;
  constructor(code: string, message: string, kind: ErrorKind | null = null, retryable = false) {
    super(message);
    this.code = code;
    this.kind = kind;
    this.retryable = retryable;
  }
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const inTauri = () => typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

async function viaTauri<T>(method: string, args: unknown): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    return (await invoke("api", { method, args })) as T;
  } catch (e) {
    const err = e as { code?: string; kind?: ErrorKind; retryable?: boolean; message?: string } | string;
    if (typeof err === "string") throw new ApiError("state", err);
    throw new ApiError(err.code ?? "state", err.message ?? "Something went wrong.", err.kind ?? null, err.retryable ?? false);
  }
}

async function viaHttp<T>(method: string, args: unknown): Promise<T> {
  let res: Response;
  try {
    res = await fetch(`/api/${method}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(args ?? {}),
    });
  } catch {
    throw new ApiError("offline", "The simulation is not reachable. Check that the server is running.");
  }
  const text = await res.text();
  let body: unknown = null;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    /* fallthrough */
  }
  if (!res.ok) {
    const e = (body as { error?: { code?: string; kind?: ErrorKind; retryable?: boolean; message?: string } } | null)?.error;
    throw new ApiError(e?.code ?? "state", e?.message ?? `Request failed (${res.status}).`, e?.kind ?? null, e?.retryable ?? false);
  }
  return body as T;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function call<T = any>(method: string, args: Record<string, unknown> = {}): Promise<T> {
  return inTauri() ? viaTauri<T>(method, args) : viaHttp<T>(method, args);
}

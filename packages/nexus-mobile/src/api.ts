// ── Nexus Mobile — API helpers ────────────────────────────────────────────────

import { Session } from "./types";

export function apiBase(session: Session): string {
  return session.serverUrl.replace(/\/$/, "") + "/api/v1";
}

export function authHeaders(session: Session): Record<string, string> {
  return {
    "Content-Type": "application/json",
    Authorization: `Bearer ${session.accessToken}`,
  };
}

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
    this.name = "ApiError";
  }
}

export async function apiFetch<T>(
  session: Session,
  path: string,
  init: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${apiBase(session)}${path}`, {
    ...init,
    headers: { ...authHeaders(session), ...(init.headers ?? {}) },
  });
  if (!res.ok) {
    const text = await res.text().catch(() => `HTTP ${res.status}`);
    throw new ApiError(res.status, text || `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

export async function apiPost<T>(
  session: Session,
  path: string,
  body: unknown
): Promise<T> {
  return apiFetch(session, path, {
    method: "POST",
    body: JSON.stringify(body),
  });
}

export async function apiPatch<T>(
  session: Session,
  path: string,
  body: unknown
): Promise<T> {
  return apiFetch(session, path, {
    method: "PATCH",
    body: JSON.stringify(body),
  });
}

export async function apiDelete(session: Session, path: string): Promise<void> {
  await fetch(`${apiBase(session)}${path}`, {
    method: "DELETE",
    headers: authHeaders(session),
  });
}

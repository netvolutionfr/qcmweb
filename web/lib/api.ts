import type { components } from "./openapi";

export type Group = components["schemas"]["Group"];
export type IssuedParticipant = components["schemas"]["IssuedParticipant"];

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

/**
 * Appel de l'API, toujours en même origine : le proxy Next.js en développement,
 * Nginx en production. Les cookies suivent donc sans configuration CORS.
 */
async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: { "content-type": "application/json", ...init?.headers },
  });

  if (!response.ok) {
    const body = await response.json().catch(() => null);
    throw new ApiError(response.status, body?.error ?? `Erreur ${response.status}`);
  }
  return response.status === 204 ? (null as T) : ((await response.json()) as T);
}

const post = <T,>(path: string, body?: unknown) =>
  call<T>(path, { method: "POST", body: body === undefined ? undefined : JSON.stringify(body) });

export const api = {
  login: (username: string, password: string) =>
    post<{ authenticated: boolean }>("/api/auth/login", { username, password }),

  logout: () => post<unknown>("/api/auth/logout"),

  me: () => call<{ authenticated: boolean }>("/api/auth/me"),

  groups: () => call<Group[]>("/api/groups"),

  createGroup: (label: string, school_year: string) =>
    post<Group>("/api/groups", { label, school_year }),

  issueTokens: (groupId: string, count: number) =>
    post<IssuedParticipant[]>(`/api/groups/${groupId}/participants`, { count }),
};

/** Année scolaire courante, bascule au 1er août. */
export function currentSchoolYear(now = new Date()): string {
  const y = now.getFullYear();
  const start = now.getMonth() >= 7 ? y : y - 1;
  return `${start}-${start + 1}`;
}

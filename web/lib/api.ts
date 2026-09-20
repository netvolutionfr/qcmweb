import type { components } from "./openapi";

export type Group = components["schemas"]["Group"];
export type IssuedParticipant = components["schemas"]["IssuedParticipant"];
export type Subject = components["schemas"]["Subject"];
export type SubjectDetail = components["schemas"]["SubjectDetail"];
export type SubjectVersion = components["schemas"]["SubjectVersion"];
export type QcmDocument = components["schemas"]["Document"];
export type Question = components["schemas"]["Question"];
export type Choice = components["schemas"]["Choice"];
export type Assessment = components["schemas"]["Assessment"];
export type ResultsTable = components["schemas"]["ResultsTable"];
export type ResultRow = components["schemas"]["ResultRow"];

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

  subjects: () => call<Subject[]>("/api/subjects"),

  subject: (id: string) => call<SubjectDetail>(`/api/subjects/${id}`),

  /**
   * Document complet d'une version : bonnes réponses et explications comprises.
   *
   * Réservé à l'enseignant qui relit. Le parcours élève passera par un tout
   * autre point d'entrée, où le serveur retire ces champs avant l'envoi
   * (SPEC §12) — les masquer à l'affichage ne serait pas une protection.
   */
  subjectDocument: (id: string, version: number) =>
    call<QcmDocument>(`/api/subjects/${id}/versions/${version}`),

  validateVersion: (id: string, version: number) =>
    post<SubjectVersion>(`/api/subjects/${id}/versions/${version}/validate`),

  archiveSubject: (id: string, archived: boolean) =>
    post<Subject>(`/api/subjects/${id}/archive`, { archived }),

  assessments: () => call<Assessment[]>("/api/assessments"),

  assessment: (id: string) => call<Assessment>(`/api/assessments/${id}`),

  openAssessment: (id: string) => post<Assessment>(`/api/assessments/${id}/open`),

  closeAssessment: (id: string) => post<Assessment>(`/api/assessments/${id}/close`),

  /** Résultats pseudonymisés : des jetons, jamais des noms. */
  results: (id: string) => call<ResultsTable>(`/api/assessments/${id}/results`),
};

/** Année scolaire courante, bascule au 1er août. */
export function currentSchoolYear(now = new Date()): string {
  const y = now.getFullYear();
  const start = now.getMonth() >= 7 ? y : y - 1;
  return `${start}-${start + 1}`;
}

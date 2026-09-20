/**
 * File d'attente des réponses non transmises.
 *
 * Le serveur reste la référence (SPEC §12) : ce stockage ne conserve jamais un
 * score ni un horodatage, seulement des réponses en attente d'envoi, le temps
 * d'une coupure réseau. À la reconnexion, elles repartent et la file se vide.
 *
 * Tout accès est protégé : en navigation privée ou avec un stockage désactivé,
 * `localStorage` lève au lieu de renvoyer null. Une épreuve ne doit pas
 * s'interrompre parce qu'un cache est indisponible.
 */

export type Pending = Record<string, string[]>;

const key = (attempt: string) => `qcmweb.pending.${attempt}`;

export function readPending(attempt: string): Pending {
  try {
    const raw = localStorage.getItem(key(attempt));
    return raw ? (JSON.parse(raw) as Pending) : {};
  } catch {
    return {};
  }
}

export function queue(attempt: string, question: string, choices: string[]): void {
  try {
    const pending = readPending(attempt);
    pending[question] = choices;
    localStorage.setItem(key(attempt), JSON.stringify(pending));
  } catch {
    // Stockage indisponible : la réponse reste en mémoire dans la page, et
    // l'indicateur d'enregistrement signalera l'échec.
  }
}

export function clearQuestion(attempt: string, question: string): void {
  try {
    const pending = readPending(attempt);
    delete pending[question];
    if (Object.keys(pending).length === 0) localStorage.removeItem(key(attempt));
    else localStorage.setItem(key(attempt), JSON.stringify(pending));
  } catch {
    /* ignoré */
  }
}

export function clearAttempt(attempt: string): void {
  try {
    localStorage.removeItem(key(attempt));
  } catch {
    /* ignoré */
  }
}

/**
 * Décalage entre l'horloge du navigateur et celle du serveur.
 *
 * L'horloge d'un poste de salle informatique est régulièrement fausse, parfois
 * de plusieurs minutes. Le compte à rebours est donc calculé à partir de
 * l'heure serveur reçue au démarrage, corrigée du temps écoulé localement.
 */
export function clockOffset(serverTime: string, receivedAt = Date.now()): number {
  return new Date(serverTime).getTime() - receivedAt;
}

export function remainingSeconds(
  deadline: string | null | undefined,
  offset: number,
  now = Date.now(),
): number | null {
  if (!deadline) return null;
  return Math.max(0, Math.round((new Date(deadline).getTime() - (now + offset)) / 1000));
}

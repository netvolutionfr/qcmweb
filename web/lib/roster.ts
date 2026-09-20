/**
 * Lecture de la liste nominative locale et jointure avec les jetons.
 *
 * Tout ce module s'exécute dans le navigateur de l'enseignant. Les noms qu'il
 * manipule ne sont jamais transmis au serveur : c'est l'invariant central du
 * projet (docs/adr/0001-pseudonymisation-par-jetons.md). Aucune fonction d'ici
 * ne doit donc jamais appeler `fetch`.
 */

export type Student = { lastName: string; firstName: string };
export type IssuedToken = { id: string; token: string; secret: string };
export type Slip = Student & IssuedToken;

/**
 * Découpe une ligne CSV en respectant les champs entre guillemets.
 *
 * Un simple `split(separator)` casserait sur `"Dupont, Jean"`, qui est
 * exactement ce que produit un export de logiciel de vie scolaire.
 */
function splitLine(line: string, separator: string): string[] {
  const fields: string[] = [];
  let current = "";
  let quoted = false;

  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (quoted) {
      if (c === '"') {
        if (line[i + 1] === '"') {
          current += '"'; // guillemet échappé par doublement
          i++;
        } else {
          quoted = false;
        }
      } else {
        current += c;
      }
    } else if (c === '"') {
      quoted = true;
    } else if (c === separator) {
      fields.push(current.trim());
      current = "";
    } else {
      current += c;
    }
  }
  fields.push(current.trim());
  return fields;
}

/** Devine le séparateur : `;` en France, `,` à l'anglo-saxonne, tabulation. */
function guessSeparator(header: string): string {
  const counts = [";", ",", "\t"].map((s) => [s, header.split(s).length] as const);
  counts.sort((a, b) => b[1] - a[1]);
  return counts[0][1] > 1 ? counts[0][0] : ";";
}

const LAST = ["nom", "nom de famille", "lastname", "last name", "name"];
const FIRST = ["prenom", "prénom", "firstname", "first name", "given name"];

function normalizeHeader(h: string): string {
  return h
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .trim();
}

/**
 * Lit un CSV nominatif.
 *
 * Tolérant par conception : BOM, fins de ligne Windows, en-têtes absents,
 * colonnes dans un ordre quelconque. Un enseignant exporte depuis l'outil qu'il
 * a sous la main, ce n'est pas à lui de se conformer à notre format.
 */
export function parseRoster(text: string): Student[] {
  const clean = text.replace(/^﻿/, "").replace(/\r\n?/g, "\n");
  const lines = clean.split("\n").filter((l) => l.trim() !== "");
  if (lines.length === 0) return [];

  const separator = guessSeparator(lines[0]);
  const first = splitLine(lines[0], separator).map(normalizeHeader);

  let lastIdx = first.findIndex((h) => LAST.includes(h));
  let firstIdx = first.findIndex((h) => FIRST.includes(h));
  const hasHeader = lastIdx !== -1 || firstIdx !== -1;

  // Sans en-tête reconnaissable, on suppose « nom ; prénom » — l'ordre usuel
  // d'une liste de classe française.
  if (!hasHeader) {
    lastIdx = 0;
    firstIdx = 1;
  }

  return lines
    .slice(hasHeader ? 1 : 0)
    .map((line) => {
      const f = splitLine(line, separator);
      return {
        lastName: f[lastIdx] ?? "",
        firstName: firstIdx === -1 ? "" : (f[firstIdx] ?? ""),
      };
    })
    .filter((s) => s.lastName !== "" || s.firstName !== "");
}

/**
 * Associe chaque élève au jeton de même rang.
 *
 * Refuse si les effectifs diffèrent : un décalage silencieux distribuerait à
 * chaque élève le billet de son voisin, et l'erreur ne serait découverte
 * qu'après l'évaluation, quand les copies seraient déjà attribuées à tort.
 */
export function joinTokens(students: Student[], tokens: IssuedToken[]): Slip[] {
  if (students.length !== tokens.length) {
    throw new Error(
      `${students.length} élèves pour ${tokens.length} jetons : refus de joindre au risque de décaler toute la liste.`,
    );
  }
  return students.map((s, i) => ({ ...s, ...tokens[i] }));
}

/**
 * Forme canonique d'un jeton : majuscules, sans séparateur.
 *
 * Le tiret n'existe que pour la lisibilité du billet. Comparer les formes
 * affichées ferait échouer la jointure au moindre écart de saisie.
 */
export function canonicalToken(token: string): string {
  return token.replace(/[^a-zA-Z0-9]/g, "").toUpperCase();
}

/**
 * Relit une table de correspondance précédemment téléchargée.
 *
 * C'est l'autre moitié de l'invariant : les noms reviennent dans le navigateur
 * le temps d'un affichage, et n'y sont jamais renvoyés au serveur.
 */
export function parseCorrespondence(text: string): Map<string, Student> {
  const clean = text.replace(/^\uFEFF/, "").replace(/\r\n?/g, "\n");
  const lines = clean.split("\n").filter((l) => l.trim() !== "");
  const out = new Map<string, Student>();
  if (lines.length === 0) return out;

  const separator = guessSeparator(lines[0]);
  const header = splitLine(lines[0], separator).map(normalizeHeader);
  const column = (names: string[]) => header.findIndex((h) => names.includes(h));

  const lastIdx = column(LAST);
  const firstIdx = column(FIRST);
  const tokenIdx = column(["jeton", "token"]);

  if (tokenIdx === -1) {
    throw new Error(
      "Ce fichier ne contient pas de colonne « jeton ». Utilisez la table de correspondance téléchargée lors de la création des billets.",
    );
  }

  for (const line of lines.slice(1)) {
    const fields = splitLine(line, separator);
    const token = canonicalToken(fields[tokenIdx] ?? "");
    if (token === "") continue;
    out.set(token, {
      lastName: lastIdx === -1 ? "" : (fields[lastIdx] ?? ""),
      firstName: firstIdx === -1 ? "" : (fields[firstIdx] ?? ""),
    });
  }
  return out;
}

/** Table de correspondance à conserver par l'enseignant. */
export function rosterCsv(slips: Slip[]): string {
  const escape = (v: string) => (/[";\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);
  const rows = slips.map((s) =>
    [s.lastName, s.firstName, s.token, s.secret].map(escape).join(";"),
  );
  return ["nom;prenom;jeton;secret", ...rows].join("\n");
}

/**
 * Planche de billets en PDF, destinée à être imprimée puis découpée.
 *
 * Le PDF est composé au millimètre plutôt que rendu depuis le DOM : les billets
 * ont ainsi une taille identique et prévisible, indépendante du navigateur, de
 * ses marges et de son zoom. Le texte reste du texte, donc net à l'impression
 * et sélectionnable — une capture rasterisée de la page donnerait des codes
 * flous, ce qui est le pire défaut possible pour des caractères à recopier.
 *
 * Comme le reste du module `roster`, tout s'exécute dans le navigateur de
 * l'enseignant : le PDF contient des noms et ne transite par aucun serveur.
 */

import { jsPDF } from "jspdf";
import type { Slip } from "./roster";

/** A4 portrait, en millimètres. */
const PAGE = { width: 210, height: 297, margin: 12 };
const GRID = { columns: 2, rows: 6, gap: 5 };

const SLIP = {
  width: (PAGE.width - 2 * PAGE.margin - (GRID.columns - 1) * GRID.gap) / GRID.columns,
  height: (PAGE.height - 2 * PAGE.margin - (GRID.rows - 1) * GRID.gap) / GRID.rows,
};

export const PER_PAGE = GRID.columns * GRID.rows;

/** Position d'un billet sur la planche, d'après son rang. */
export function place(index: number) {
  const onPage = index % PER_PAGE;
  const row = Math.floor(onPage / GRID.columns);
  const column = onPage % GRID.columns;

  return {
    page: Math.floor(index / PER_PAGE),
    x: PAGE.margin + column * (SLIP.width + GRID.gap),
    y: PAGE.margin + row * (SLIP.height + GRID.gap),
    width: SLIP.width,
    height: SLIP.height,
  };
}

export function pageCount(total: number): number {
  return Math.max(1, Math.ceil(total / PER_PAGE));
}

export type PdfGroup = { label: string; school_year: string };

export function buildBillets(slips: Slip[], group?: PdfGroup): Blob {
  const doc = new jsPDF({ unit: "mm", format: "a4" });
  const pages = pageCount(slips.length);

  slips.forEach((slip, index) => {
    const box = place(index);
    if (box.page > 0 && index % PER_PAGE === 0) doc.addPage();

    // Trait de découpe : discret, mais il faut voir où couper.
    doc.setDrawColor(190);
    doc.setLineWidth(0.2);
    doc.rect(box.x, box.y, box.width, box.height);

    const left = box.x + 6;
    let y = box.y + 9;

    doc.setTextColor(20);
    doc.setFont("helvetica", "bold");
    doc.setFontSize(12);
    doc.text(`${slip.firstName} ${slip.lastName}`.trim(), left, y);

    if (group) {
      y += 5;
      doc.setFont("helvetica", "normal");
      doc.setFontSize(8.5);
      doc.setTextColor(120);
      doc.text(`${group.label} — ${group.school_year}`, left, y);
    }

    // Courier pour les codes : chasse fixe, donc pas de confusion entre
    // glyphes voisins au moment de la recopie.
    y += 9;
    for (const [caption, value] of [
      ["jeton", slip.token],
      ["secret", slip.secret],
    ] as const) {
      doc.setFont("helvetica", "normal");
      doc.setFontSize(8);
      doc.setTextColor(120);
      doc.text(caption, left, y);

      doc.setFont("courier", "bold");
      doc.setFontSize(12);
      doc.setTextColor(20);
      doc.text(value, left + 16, y);
      y += 7;
    }
  });

  for (let page = 1; page <= pages; page++) {
    doc.setPage(page);
    doc.setFont("helvetica", "normal");
    doc.setFontSize(7.5);
    doc.setTextColor(150);
    doc.text(
      `${group ? `${group.label} · ` : ""}page ${page}/${pages} · à découper et distribuer`,
      PAGE.margin,
      PAGE.height - 6,
    );
  }

  return doc.output("blob");
}

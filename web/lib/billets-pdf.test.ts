import { test } from "node:test";
import assert from "node:assert/strict";
import { place, pageCount, PER_PAGE } from "./billets-pdf.ts";

test("tous les billets ont exactement la meme taille", () => {
  const sizes = new Set(
    Array.from({ length: 40 }, (_, i) => {
      const b = place(i);
      return `${b.width.toFixed(3)}x${b.height.toFixed(3)}`;
    }),
  );
  assert.equal(sizes.size, 1, `tailles distinctes : ${[...sizes].join(", ")}`);
});

test("la planche tient dans une A4 avec ses marges", () => {
  const last = place(PER_PAGE - 1);
  assert.ok(last.x + last.width <= 210 - 12 + 0.01, "déborde en largeur");
  assert.ok(last.y + last.height <= 297 - 12 + 0.01, "déborde en hauteur");
});

test("remplissage en lignes, de gauche a droite", () => {
  const a = place(0);
  const b = place(1);
  const c = place(2);
  assert.equal(a.y, b.y, "les deux premiers sont sur la même ligne");
  assert.ok(b.x > a.x, "le second est à droite du premier");
  assert.ok(c.y > a.y, "le troisième passe à la ligne");
  assert.equal(c.x, a.x, "et revient en première colonne");
});

test("aucun chevauchement entre billets voisins", () => {
  const a = place(0);
  const b = place(1);
  assert.ok(a.x + a.width <= b.x, "chevauchement horizontal");
  const c = place(PER_PAGE - 2);
  const d = place(PER_PAGE - 1);
  assert.ok(c.x + c.width <= d.x);
});

test("pagination", () => {
  assert.equal(pageCount(0), 1);
  assert.equal(pageCount(1), 1);
  assert.equal(pageCount(PER_PAGE), 1);
  assert.equal(pageCount(PER_PAGE + 1), 2);
  assert.equal(place(PER_PAGE).page, 1);
  assert.equal(place(PER_PAGE).y, place(0).y, "la nouvelle page repart en haut");
});

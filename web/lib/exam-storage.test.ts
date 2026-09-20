import { test } from "node:test";
import assert from "node:assert/strict";
import { clockOffset, remainingSeconds } from "./exam-storage.ts";

test("decalage d'horloge mesure", () => {
  const received = Date.parse("2026-09-20T10:00:00Z");
  // Le serveur est en avance de deux minutes sur le poste.
  assert.equal(clockOffset("2026-09-20T10:02:00Z", received), 120_000);
});

test("le compte a rebours suit l'heure serveur, pas celle du poste", () => {
  const now = Date.parse("2026-09-20T10:00:00Z");
  const offset = clockOffset("2026-09-20T10:05:00Z", now); // poste en retard de 5 min
  const left = remainingSeconds("2026-09-20T10:35:00Z", offset, now);
  assert.equal(left, 30 * 60, "trente minutes restantes selon le serveur");
});

test("sans echeance, pas de compte a rebours", () => {
  assert.equal(remainingSeconds(null, 0), null);
  assert.equal(remainingSeconds(undefined, 0), null);
});

test("le compte a rebours ne devient jamais negatif", () => {
  const now = Date.parse("2026-09-20T11:00:00Z");
  assert.equal(remainingSeconds("2026-09-20T10:00:00Z", 0, now), 0);
});

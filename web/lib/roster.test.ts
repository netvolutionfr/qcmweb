import { test } from "node:test";
import assert from "node:assert/strict";
import { parseRoster, joinTokens, rosterCsv } from "./roster.ts";

test("en-tetes reconnus quel que soit l'ordre et la casse", () => {
  const r = parseRoster("Prénom;NOM\nAlice;Martin\nBob;Dupont");
  assert.deepEqual(r, [
    { firstName: "Alice", lastName: "Martin" },
    { firstName: "Bob", lastName: "Dupont" },
  ]);
});

test("sans en-tete, on suppose nom puis prenom", () => {
  const r = parseRoster("Martin;Alice\nDupont;Bob");
  assert.equal(r.length, 2);
  assert.deepEqual(r[0], { lastName: "Martin", firstName: "Alice" });
});

test("champs entre guillemets contenant le separateur", () => {
  const r = parseRoster('nom;prenom\n"Dupont, de la Tour";Jean');
  assert.equal(r[0].lastName, "Dupont, de la Tour");
});

test("guillemet echappe par doublement", () => {
  const r = parseRoster('nom;prenom\n"L""Hermite";Marie');
  assert.equal(r[0].lastName, 'L"Hermite');
});

test("BOM, CRLF et lignes vides", () => {
  const r = parseRoster('﻿nom;prenom\r\nMartin;Alice\r\n\r\nDupont;Bob\r\n');
  assert.equal(r.length, 2);
  assert.equal(r[0].lastName, "Martin");
});

test("separateur virgule detecte", () => {
  const r = parseRoster("nom,prenom\nMartin,Alice");
  assert.deepEqual(r[0], { lastName: "Martin", firstName: "Alice" });
});

test("jointure refusee si les effectifs different", () => {
  const students = [{ lastName: "Martin", firstName: "Alice" }];
  assert.throws(() => joinTokens(students, []), /refus de joindre/);
});

test("jointure par rang", () => {
  const slips = joinTokens(
    [
      { lastName: "Martin", firstName: "Alice" },
      { lastName: "Dupont", firstName: "Bob" },
    ],
    [
      { id: "1", token: "AAAA-BBBB", secret: "S1" },
      { id: "2", token: "CCCC-DDDD", secret: "S2" },
    ],
  );
  assert.equal(slips[0].token, "AAAA-BBBB");
  assert.equal(slips[1].lastName, "Dupont");
});

test("csv de correspondance echappe les champs sensibles", () => {
  const csv = rosterCsv([
    { lastName: 'Dupont; "le grand"', firstName: "Jean", id: "1", token: "T", secret: "S" },
  ]);
  assert.match(csv, /^nom;prenom;jeton;secret\n/);
  assert.match(csv, /"Dupont; ""le grand"""/);
});

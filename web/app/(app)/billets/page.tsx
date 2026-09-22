"use client";

import { useEffect, useState } from "react";
import { Download, FileDown, ShieldCheck, TriangleAlert, Upload } from "lucide-react";
import { api, currentSchoolYear, type Group } from "@/lib/api";
import { joinTokens, parseRoster, rosterCsv, type Slip, type Student } from "@/lib/roster";
import { BilletSheet } from "@/components/billets/billet-sheet";
import { buildBillets, pageCount } from "@/lib/billets-pdf";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export default function Billets() {
  const [groups, setGroups] = useState<Group[]>([]);
  const [groupId, setGroupId] = useState("");
  const [newLabel, setNewLabel] = useState("");
  const [students, setStudents] = useState<Student[]>([]);
  const [fileName, setFileName] = useState("");
  const [slips, setSlips] = useState<Slip[] | null>(null);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const group = groups.find((g) => g.id === groupId);

  useEffect(() => {
    api.groups().then(setGroups).catch(fail);
  }, []);

  // Les secrets n'existent qu'en mémoire. Un rechargement les perd
  // définitivement : il faudrait alors réinitialiser chaque jeton un par un.
  useEffect(() => {
    if (!slips || saved) return;
    const warn = (e: BeforeUnloadEvent) => e.preventDefault();
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [slips, saved]);

  function fail(err: unknown) {
    setError(err instanceof Error ? err.message : String(err));
  }

  async function createGroup() {
    setError(null);
    try {
      const created = await api.createGroup(newLabel.trim(), currentSchoolYear());
      setGroups((previous) => [created, ...previous]);
      setGroupId(created.id);
      setNewLabel("");
    } catch (err) {
      fail(err);
    }
  }

  async function readRoster(file: File) {
    setError(null);
    setSlips(null);
    try {
      // Lecture strictement locale : le fichier n'est envoyé nulle part.
      const parsed = parseRoster(await file.text());
      if (parsed.length === 0) throw new Error("Aucun élève trouvé dans ce fichier.");
      setStudents(parsed);
      setFileName(file.name);
    } catch (err) {
      setStudents([]);
      setFileName("");
      fail(err);
    }
  }

  async function generate() {
    setBusy(true);
    setError(null);
    try {
      const issued = await api.issueTokens(groupId, students.length);
      setSlips(joinTokens(students, issued));
      setSaved(false);
    } catch (err) {
      fail(err);
    } finally {
      setBusy(false);
    }
  }

  function save(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.click();
    URL.revokeObjectURL(url);
  }

  const stem = `${group?.label ?? "groupe"}-${group?.school_year ?? ""}`;

  function downloadCsv() {
    if (!slips) return;
    save(new Blob([rosterCsv(slips)], { type: "text/csv;charset=utf-8" }), `correspondance-${stem}.csv`);
    setSaved(true);
  }

  function downloadPdf() {
    if (!slips) return;
    save(buildBillets(slips, group), `billets-${stem}.pdf`);
  }

  return (
    <div className="space-y-6">
      <div className="print:hidden">
        <h1 className="text-2xl font-semibold tracking-tight">Billets de participation</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Générer les jetons d&apos;un groupe et imprimer les billets à distribuer.
        </p>
      </div>

      <Alert className="print:hidden">
        <ShieldCheck />
        <AlertTitle>Votre liste ne quitte pas cet ordinateur</AlertTitle>
        <AlertDescription>
          Le fichier nominatif est lu dans ce navigateur et joint aux jetons ici même. Le serveur ne
          connaîtra que des jetons, jamais un nom.
        </AlertDescription>
      </Alert>

      {error && (
        <Alert variant="destructive" className="print:hidden">
          <TriangleAlert />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Card className="print:hidden">
        <CardHeader>
          <CardTitle className="text-base">1. Choisir le groupe</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <Select value={groupId} onValueChange={(value) => setGroupId(value ?? "")}>
            <SelectTrigger className="w-full">
              {/* Base UI n'affiche le libellé d'un item qu'avec la prop `items` sur
                  `Select.Root`, ou — comme ici — une fonction de rendu sur `Value` :
                  sans elle, il affiche la valeur brute, c'est-à-dire l'UUID du groupe. */}
              <SelectValue placeholder="Sélectionner un groupe">
                {() => {
                  const g = groups.find((g) => g.id === groupId);
                  return g ? `${g.label} · ${g.school_year}` : "Sélectionner un groupe";
                }}
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              {groups.map((g) => (
                <SelectItem key={g.id} value={g.id}>
                  {g.label} · {g.school_year} · {g.participants} jeton
                  {g.participants > 1 ? "s" : ""}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
            <div className="flex-1 space-y-2">
              <Label htmlFor="label">Ou créer un groupe pour {currentSchoolYear()}</Label>
              <Input
                id="label"
                value={newLabel}
                placeholder="1SIO"
                onChange={(e) => setNewLabel(e.target.value)}
              />
            </div>
            <Button variant="secondary" onClick={createGroup} disabled={newLabel.trim() === ""}>
              Créer
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card className="print:hidden">
        <CardHeader>
          <CardTitle className="text-base">2. Charger votre liste</CardTitle>
          <CardDescription>
            Un CSV avec une colonne nom et une colonne prénom. Séparateur, accents et guillemets
            sont détectés automatiquement.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Label
            htmlFor="roster"
            className="flex cursor-pointer items-center gap-2 rounded-md border border-dashed px-4 py-6 text-sm text-muted-foreground hover:bg-accent"
          >
            <Upload className="size-4 shrink-0" />
            {fileName || "Choisir un fichier CSV"}
          </Label>
          {/* Champ natif, pas le composant Input : ses classes de base (dont
             `w-full`) l'emportent sur `sr-only` dans la feuille compilée — même
             spécificité, `w-full` déclaré après — et un champ en position:absolute
             hérite alors d'une largeur de 100 % du viewport plutôt que de 1px,
             provoquant un débordement horizontal invisible mais bien réel. */}
          <input
            id="roster"
            type="file"
            accept=".csv,text/csv,text/plain"
            className="sr-only"
            onChange={(e) => e.target.files?.[0] && readRoster(e.target.files[0])}
          />

          {students.length > 0 && (
            <p className="text-sm">
              <Badge variant="secondary">{students.length} élèves</Badge>{" "}
              <span className="text-muted-foreground">
                de {students[0].lastName} {students[0].firstName} à{" "}
                {students[students.length - 1].lastName} {students[students.length - 1].firstName}
              </span>
            </p>
          )}
        </CardContent>
      </Card>

      <Card className="print:hidden">
        <CardHeader>
          <CardTitle className="text-base">3. Générer les jetons</CardTitle>
          <CardDescription>
            Un jeton et un secret par élève, valables toute l&apos;année scolaire.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Button
            onClick={generate}
            disabled={busy || !groupId || students.length === 0}
            className="w-full sm:w-auto"
          >
            {busy ? "Génération en cours…" : `Générer ${students.length || ""} jetons`}
          </Button>
          {busy && (
            <p className="mt-2 text-sm text-muted-foreground">
              Chaque secret est chiffré séparément : comptez quelques secondes pour une classe.
            </p>
          )}
        </CardContent>
      </Card>

      {slips && (
        <>
          <Card className="print:hidden">
            <CardHeader>
              <CardTitle className="text-base">4. Conserver, puis imprimer</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <Alert variant={saved ? "default" : "destructive"}>
                <TriangleAlert />
                <AlertTitle>
                  {saved
                    ? "Table de correspondance enregistrée"
                    : "Les secrets ne sont affichés qu'une fois"}
                </AlertTitle>
                <AlertDescription>
                  {saved
                    ? "Sauvegardez-la ailleurs : sans elle, les résultats ne seront plus attribuables à personne."
                    : "Téléchargez la table de correspondance avant de quitter cette page. Le serveur n'en conserve qu'une empreinte et ne pourra jamais les réafficher."}
                </AlertDescription>
              </Alert>

              <div className="flex flex-col gap-2 sm:flex-row">
                <Button onClick={downloadCsv}>
                  <Download className="size-4" />
                  Télécharger la correspondance
                </Button>
                <Button variant="secondary" onClick={downloadPdf}>
                  <FileDown className="size-4" />
                  Billets en PDF
                  <span className="text-muted-foreground">
                    ({pageCount(slips.length)} page{pageCount(slips.length) > 1 ? "s" : ""})
                  </span>
                </Button>
              </div>
            </CardContent>
          </Card>

          <BilletSheet slips={slips} group={group} />
        </>
      )}
    </div>
  );
}

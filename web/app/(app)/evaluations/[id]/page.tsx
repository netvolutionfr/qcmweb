"use client";

import { use, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import {
  ArrowLeft,
  Download,
  Lock,
  LockOpen,
  ShieldCheck,
  TriangleAlert,
  Upload,
} from "lucide-react";
import { api, type Assessment, type ResultsTable } from "@/lib/api";
import { canonicalToken, parseCorrespondence, type Student } from "@/lib/roster";
import { StateBadge } from "@/components/evaluations/state-badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

const STANDING = {
  SUBMITTED: "remis",
  IN_PROGRESS: "en cours",
  ABSENT: "absent",
} as const;

function duration(seconds: number | null | undefined) {
  if (seconds == null) return "—";
  const m = Math.floor(seconds / 60);
  return `${m}:${String(seconds % 60).padStart(2, "0")}`;
}

export default function Resultats({ params }: { params: Promise<{ id: string }> }) {
  const { id } = use(params);

  const [assessment, setAssessment] = useState<Assessment | null>(null);
  const [table, setTable] = useState<ResultsTable | null>(null);
  const [names, setNames] = useState<Map<string, Student> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useMemo(
    () => () =>
      Promise.all([api.assessment(id), api.results(id)])
        .then(([a, t]) => {
          setAssessment(a);
          setTable(t);
        })
        .catch((err) => setError(err instanceof Error ? err.message : String(err))),
    [id],
  );

  useEffect(() => {
    reload();
  }, [reload]);

  async function toggle() {
    if (!assessment) return;
    setBusy(true);
    setError(null);
    try {
      const next =
        assessment.state === "OPEN"
          ? await api.closeAssessment(id)
          : await api.openAssessment(id);
      setAssessment(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function loadNames(file: File) {
    setError(null);
    try {
      // Lecture locale : la table de correspondance ne repart nulle part.
      setNames(parseCorrespondence(await file.text()));
    } catch (err) {
      setNames(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  function save(content: string, filename: string, type = "text/csv;charset=utf-8") {
    const url = URL.createObjectURL(new Blob([content], { type }));
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    link.click();
    URL.revokeObjectURL(url);
  }

  /** Export nominatif, composé ici : le serveur n'a jamais vu ces noms. */
  function exportNominatif() {
    if (!table || !names) return;
    const escape = (v: string) => (/[";\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);
    const lines = table.rows.map((row) => {
      const who = names.get(canonicalToken(row.token));
      return [
        escape(who?.lastName ?? ""),
        escape(who?.firstName ?? ""),
        row.token,
        STANDING[row.standing],
        row.score ?? "",
        row.max_score ?? "",
        row.percentage ?? "",
        row.grade ?? "",
      ].join(";");
    });
    save(
      "﻿nom;prenom;jeton;etat;score;bareme;pourcentage;note\n" + lines.join("\n"),
      `resultats-nominatif-${assessment?.name ?? ""}.csv`,
    );
  }

  const unmatched = useMemo(() => {
    if (!table || !names) return 0;
    return table.rows.filter((r) => !names.get(canonicalToken(r.token))).length;
  }, [table, names]);

  if (!assessment || !table) {
    return error ? (
      <Alert variant="destructive">
        <TriangleAlert />
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    ) : (
      <p className="text-sm text-muted-foreground">Chargement…</p>
    );
  }

  return (
    <div className="space-y-6">
      <Link
        href="/evaluations"
        className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Toutes les évaluations
      </Link>

      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold tracking-tight">{assessment.name}</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {assessment.group_label} · {assessment.subject_title} v{assessment.subject_version}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className="font-mono text-base tracking-widest">
            {assessment.code}
          </Badge>
          <StateBadge state={assessment.state} available={assessment.available} />
        </div>
      </div>

      {error && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {assessment.state === "OPEN" && !assessment.available && (
        <Alert>
          <TriangleAlert />
          <AlertTitle>Ouverte, mais hors de sa fenêtre</AlertTitle>
          <AlertDescription>
            Les élèves ne peuvent pas composer pour l&apos;instant. Vérifiez les dates
            d&apos;ouverture et de fermeture.
          </AlertDescription>
        </Alert>
      )}

      <div className="flex flex-col gap-2 sm:flex-row">
        <Button onClick={toggle} disabled={busy} variant={assessment.state === "OPEN" ? "secondary" : "default"}>
          {assessment.state === "OPEN" ? <Lock className="size-4" /> : <LockOpen className="size-4" />}
          {assessment.state === "OPEN" ? "Fermer l'évaluation" : "Ouvrir l'évaluation"}
        </Button>
        <Button variant="outline" onClick={reload}>
          Actualiser
        </Button>
      </div>

      <div className="grid grid-cols-3 gap-3">
        {[
          ["remis", table.submitted],
          ["en cours", table.in_progress],
          ["absents", table.absent],
        ].map(([label, value]) => (
          <Card key={label as string}>
            <CardContent className="py-4 text-center">
              <p className="text-2xl font-semibold">{value}</p>
              <p className="text-xs text-muted-foreground">{label}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Afficher les noms</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Alert>
            <ShieldCheck />
            <AlertDescription>
              Le serveur ne connaît que des jetons. Chargez votre table de correspondance pour
              afficher les noms : la jointure se fait dans ce navigateur, et disparaît au
              rechargement.
            </AlertDescription>
          </Alert>

          <Label
            htmlFor="corr"
            className="flex cursor-pointer items-center gap-2 rounded-md border border-dashed px-4 py-4 text-sm text-muted-foreground hover:bg-accent"
          >
            <Upload className="size-4 shrink-0" />
            {names ? `${names.size} correspondances chargées` : "Charger ma table de correspondance"}
          </Label>
          <Input
            id="corr"
            type="file"
            accept=".csv,text/csv,text/plain"
            className="sr-only"
            onChange={(e) => e.target.files?.[0] && loadNames(e.target.files[0])}
          />

          {names && unmatched > 0 && (
            <p className="text-sm text-amber-700 dark:text-amber-500">
              {unmatched} jeton{unmatched > 1 ? "s" : ""} sans correspondance — table d&apos;un
              autre groupe ou jetons régénérés depuis.
            </p>
          )}
        </CardContent>
      </Card>

      <div className="overflow-x-auto rounded-lg border">
        <table className="w-full text-sm">
          <thead className="border-b bg-muted/50 text-left">
            <tr>
              <th className="p-2 font-medium">{names ? "Élève" : "Jeton"}</th>
              <th className="p-2 font-medium">État</th>
              <th className="p-2 text-right font-medium">Score</th>
              <th className="p-2 text-right font-medium">%</th>
              <th className="p-2 text-right font-medium">Note</th>
              <th className="p-2 text-right font-medium">Durée</th>
            </tr>
          </thead>
          <tbody>
            {table.rows.map((row) => {
              const who = names?.get(canonicalToken(row.token));
              return (
                <tr key={row.participant_id} className="border-b last:border-0">
                  <td className="p-2">
                    {who ? (
                      <span>
                        {who.lastName} <span className="text-muted-foreground">{who.firstName}</span>
                      </span>
                    ) : (
                      <span className="font-mono text-xs">{row.token}</span>
                    )}
                  </td>
                  <td className="p-2 text-muted-foreground">{STANDING[row.standing]}</td>
                  <td className="p-2 text-right tabular-nums">
                    {row.score != null ? `${row.score}/${row.max_score}` : "—"}
                  </td>
                  <td className="p-2 text-right tabular-nums">{row.percentage ?? "—"}</td>
                  <td className="p-2 text-right tabular-nums">{row.grade ?? "—"}</td>
                  <td className="p-2 text-right tabular-nums">{duration(row.duration_seconds)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {table.questions.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Réussite par question</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {table.questions.map((q) => (
              <div key={q.question_id}>
                <div className="flex items-baseline justify-between gap-2 text-sm">
                  <span className="truncate font-mono text-xs">{q.question_id}</span>
                  <span className="shrink-0 tabular-nums">
                    {q.success_rate} %{" "}
                    <span className="text-muted-foreground">
                      (moyenne {q.average_rate} %)
                    </span>
                  </span>
                </div>
                <div className="mt-1 h-2 overflow-hidden rounded-full bg-muted">
                  <div
                    className="h-full rounded-full bg-foreground/70"
                    style={{ width: `${q.success_rate}%` }}
                  />
                </div>
              </div>
            ))}
            <p className="text-xs text-muted-foreground">
              Taux de réussite complète, et moyenne des points obtenus. Les deux diffèrent dès
              qu&apos;une question est notée partiellement.
            </p>
          </CardContent>
        </Card>
      )}

      <div className="flex flex-col gap-2 sm:flex-row">
        <Button variant="secondary" onClick={() => window.open(`/api/assessments/${id}/results.csv`)}>
          <Download className="size-4" />
          Export par jetons
        </Button>
        <Button onClick={exportNominatif} disabled={!names}>
          <Download className="size-4" />
          Export nominatif
        </Button>
      </div>
      {!names && (
        <p className="text-xs text-muted-foreground">
          L&apos;export nominatif est composé dans ce navigateur, après chargement de votre table
          de correspondance.
        </p>
      )}
    </div>
  );
}

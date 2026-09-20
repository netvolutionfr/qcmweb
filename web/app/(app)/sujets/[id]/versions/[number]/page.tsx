"use client";

import { use, useEffect, useState } from "react";
import Link from "next/link";
import { ArrowLeft, CheckCircle2, ShieldCheck, TriangleAlert } from "lucide-react";
import {
  api,
  type QcmDocument,
  type SubjectDetail,
} from "@/lib/api";
import { QuestionPreview, type PreviewMode } from "@/components/sujets/question-preview";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";

export default function Preview({
  params,
}: {
  params: Promise<{ id: string; number: string }>;
}) {
  const { id, number } = use(params);
  const version = Number(number);

  const [detail, setDetail] = useState<SubjectDetail | null>(null);
  const [document, setDocument] = useState<QcmDocument | null>(null);
  const [mode, setMode] = useState<PreviewMode>("eleve");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    Promise.all([api.subject(id), api.subjectDocument(id, version)])
      .then(([d, doc]) => {
        setDetail(d);
        setDocument(doc);
      })
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, [id, version]);

  const current = detail?.versions.find((v) => v.number === version);
  const validated = current?.status === "VALIDATED";

  async function approve() {
    setBusy(true);
    setError(null);
    try {
      await api.validateVersion(id, version);
      setDetail(await api.subject(id));
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  if (error && !document) {
    return (
      <Alert variant="destructive">
        <TriangleAlert />
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!document || !current) {
    return <p className="text-sm text-muted-foreground">Chargement du sujet…</p>;
  }

  return (
    <div className="space-y-6">
      <Link
        href="/sujets"
        className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Tous les sujets
      </Link>

      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold tracking-tight">{document.metadata.title}</h1>
          <p className="mt-1 flex flex-wrap items-center gap-2 text-sm text-muted-foreground">
            <span>version {current.number}</span>
            <span>·</span>
            <span>
              {current.question_count} question{current.question_count > 1 ? "s" : ""}
            </span>
            <span>·</span>
            <span>{current.total_points} points</span>
          </p>
        </div>
        <Badge variant={validated ? "default" : "secondary"}>
          {validated ? "validé" : "brouillon"}
        </Badge>
      </div>

      {document.metadata.description && (
        <p className="text-sm text-muted-foreground">{document.metadata.description}</p>
      )}

      {error && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Tabs value={mode} onValueChange={(value) => setMode(value as PreviewMode)}>
        <TabsList>
          <TabsTrigger value="eleve">Vue élève</TabsTrigger>
          <TabsTrigger value="corrige">Vue corrigée</TabsTrigger>
        </TabsList>
      </Tabs>

      <p className="text-sm text-muted-foreground">
        {mode === "eleve"
          ? "Exactement ce que verra la classe : ni bonne réponse, ni explication."
          : "Bonnes réponses, explications et barème, pour vérifier la correction."}
      </p>

      <ol className="space-y-3">
        {document.questions.map((question, index) => (
          <QuestionPreview key={question.id} question={question} index={index} mode={mode} />
        ))}
      </ol>

      <Card>
        <CardContent className="space-y-4">
          {validated ? (
            <Alert>
              <CheckCircle2 />
              <AlertTitle>Version validée</AlertTitle>
              <AlertDescription>
                Elle peut servir de base à une évaluation. Son contenu est figé : une retouche
                créera une nouvelle version, sans jamais altérer les évaluations passées.
              </AlertDescription>
            </Alert>
          ) : (
            <>
              <Alert>
                <ShieldCheck />
                <AlertTitle>La validation vous revient</AlertTitle>
                <AlertDescription>
                  Un agent peut déposer un sujet, jamais le publier. Relisez les deux vues, puis
                  validez : la version sera alors figée et utilisable pour une évaluation.
                </AlertDescription>
              </Alert>
              <Button onClick={approve} disabled={busy} className="w-full sm:w-auto">
                {busy ? "Validation…" : `Valider la version ${current.number}`}
              </Button>
            </>
          )}
        </CardContent>
      </Card>

      {detail && detail.versions.length > 1 && (
        <section>
          <h2 className="text-sm font-medium">Autres versions</h2>
          <ul className="mt-2 space-y-1">
            {detail.versions
              .filter((v) => v.number !== version)
              .map((v) => (
                <li key={v.number}>
                  <Link
                    href={`/sujets/${id}/versions/${v.number}`}
                    className="flex items-center gap-3 rounded-md border px-3 py-2 text-sm hover:bg-accent"
                  >
                    <span className="flex-1 truncate">
                      version {v.number} — {v.title}
                    </span>
                    <Badge variant={v.status === "VALIDATED" ? "default" : "secondary"}>
                      {v.status === "VALIDATED" ? "validé" : "brouillon"}
                    </Badge>
                  </Link>
                </li>
              ))}
          </ul>
        </section>
      )}
    </div>
  );
}

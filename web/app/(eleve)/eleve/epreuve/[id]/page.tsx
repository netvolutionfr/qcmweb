"use client";

import { use, useCallback, useEffect, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { CheckCircle2, CloudOff, Loader2, Send, TriangleAlert } from "lucide-react";
import { student, ApiError, type AttemptResult, type Sitting } from "@/lib/api";
import {
  clearAttempt,
  clearQuestion,
  clockOffset,
  queue,
  readPending,
} from "@/lib/exam-storage";
import { Countdown } from "@/components/eleve/countdown";
import { QuestionCard } from "@/components/eleve/question-card";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type SaveState = "idle" | "saving" | "saved" | "offline";

export default function Epreuve({ params }: { params: Promise<{ id: string }> }) {
  // L'identifiant de l'URL ne sert qu'à rendre le lien partageable. La
  // tentative qui fait foi est celle que le serveur renvoie : s'il en ouvre
  // une nouvelle — copie précédente remise, par exemple — écrire vers l'ancien
  // identifiant enverrait les réponses dans le vide.
  use(params);
  const router = useRouter();
  const [attemptId, setAttemptId] = useState<string | null>(null);
  const assessmentId = useSearchParams().get("a");

  const [sitting, setSitting] = useState<Sitting | null>(null);
  const [answers, setAnswers] = useState<Record<string, string[]>>({});
  const [offset, setOffset] = useState(0);
  const [save, setSave] = useState<SaveState>("idle");
  const [result, setResult] = useState<AttemptResult | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submitting = useRef(false);

  useEffect(() => {
    if (!assessmentId) {
      router.replace("/eleve");
      return;
    }
    student
      .sit(assessmentId)
      .then((s) => {
        setSitting(s);
        setAttemptId(s.attempt_id);
        setOffset(clockOffset(s.server_time));
        // Les réponses du serveur font foi, complétées par ce qui n'a pas pu
        // partir lors d'une coupure.
        setAnswers({ ...s.answers, ...readPending(s.attempt_id) });
      })
      .catch((err) => {
        if (err instanceof ApiError && err.status === 401) router.replace("/eleve/connexion");
        else setError(err instanceof Error ? err.message : String(err));
      });
  }, [assessmentId, router]);

  /** Envoie une réponse, et la met en file d'attente si le réseau manque. */
  const push = useCallback(
    async (question: string, choices: string[]) => {
      if (!attemptId) return;
      setSave("saving");
      try {
        await student.answer(attemptId, question, choices);
        clearQuestion(attemptId, question);
        setSave("saved");
      } catch (err) {
        if (err instanceof ApiError && err.status === 409) {
          // Temps écoulé ou copie déjà remise : le serveur tranche.
          setError("Le temps est écoulé ou la copie est déjà remise.");
          setSave("idle");
          return;
        }
        queue(attemptId, question, choices);
        setSave("offline");
      }
    },
    [attemptId],
  );

  // Reprise des envois en attente dès le retour du réseau.
  useEffect(() => {
    if (!attemptId) return;
    const retry = () => {
      const pending = readPending(attemptId);
      for (const [question, choices] of Object.entries(pending)) push(question, choices);
    };
    window.addEventListener("online", retry);
    return () => window.removeEventListener("online", retry);
  }, [attemptId, push]);

  function change(question: string, choices: string[]) {
    setAnswers((previous) => ({ ...previous, [question]: choices }));
    push(question, choices);
  }

  const submit = useCallback(async () => {
    if (submitting.current || !attemptId) return;
    submitting.current = true;
    try {
      setResult(await student.submit(attemptId));
      clearAttempt(attemptId);
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        setResult(await student.result(attemptId).catch(() => null));
      } else {
        setError(err instanceof Error ? err.message : String(err));
        submitting.current = false;
      }
    }
  }, [attemptId]);

  if (error && !sitting) {
    return (
      <Alert variant="destructive">
        <TriangleAlert />
        <AlertDescription>{error}</AlertDescription>
      </Alert>
    );
  }

  if (!sitting) {
    return <p className="text-sm text-muted-foreground">Chargement de l&apos;épreuve…</p>;
  }

  if (result) return <Resultat result={result} />;

  const answered = sitting.exam.questions.filter((q) => (answers[q.id] ?? []).length > 0).length;
  const total = sitting.exam.questions.length;

  return (
    <div className="space-y-4">
      <header className="sticky top-0 z-10 -mx-4 flex items-center gap-3 border-b bg-background/90 px-4 py-3 backdrop-blur">
        <div className="min-w-0 flex-1">
          <p className="truncate font-medium">{sitting.exam.title}</p>
          <p className="text-xs text-muted-foreground">
            {answered} / {total} répondues
          </p>
        </div>
        <SaveBadge state={save} />
        <Countdown deadline={sitting.deadline ?? null} offset={offset} onExpire={submit} />
      </header>

      {error && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {sitting.exam.description && (
        <p className="text-sm text-muted-foreground">{sitting.exam.description}</p>
      )}

      <div className="space-y-6 sm:space-y-8">
        {sitting.exam.questions.map((question, index) => (
          <QuestionCard
            key={question.id}
            question={question}
            index={index}
            total={total}
            selected={answers[question.id] ?? []}
            onChange={(choices) => change(question.id, choices)}
            disabled={confirming}
          />
        ))}
      </div>

      <Card>
        <CardContent className="space-y-4">
          {answered < total && (
            <Alert>
              <TriangleAlert />
              <AlertDescription>
                {total - answered} question{total - answered > 1 ? "s" : ""} sans réponse.
              </AlertDescription>
            </Alert>
          )}

          {confirming ? (
            <div className="space-y-3">
              <Alert variant="destructive">
                <TriangleAlert />
                <AlertTitle>Remise définitive</AlertTitle>
                <AlertDescription>
                  Tu ne pourras plus modifier tes réponses.
                </AlertDescription>
              </Alert>
              <div className="flex flex-col gap-2 sm:flex-row">
                <Button onClick={submit} size="lg" className="flex-1">
                  <Send className="size-4" />
                  Oui, je remets ma copie
                </Button>
                <Button variant="secondary" size="lg" onClick={() => setConfirming(false)}>
                  Revenir au questionnaire
                </Button>
              </div>
            </div>
          ) : (
            <Button onClick={() => setConfirming(true)} size="lg" className="w-full">
              <Send className="size-4" />
              Remettre ma copie
            </Button>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function SaveBadge({ state }: { state: SaveState }) {
  if (state === "idle") return null;

  const content = {
    saving: [<Loader2 key="i" className="size-3.5 animate-spin" />, "…"],
    saved: [<CheckCircle2 key="i" className="size-3.5" />, "enregistré"],
    offline: [<CloudOff key="i" className="size-3.5" />, "hors ligne"],
  }[state];

  return (
    <span
      aria-live="polite"
      className={`flex items-center gap-1 text-xs ${
        state === "offline" ? "text-amber-700 dark:text-amber-500" : "text-muted-foreground"
      }`}
    >
      {content[0]}
      <span className="hidden sm:inline">{content[1]}</span>
    </span>
  );
}

/** Retour à l'élève, strictement limité à ce que le serveur a bien voulu rendre. */
function Resultat({ result }: { result: AttemptResult }) {
  return (
    <div className="space-y-4">
      <Alert>
        <CheckCircle2 />
        <AlertTitle>Copie remise</AlertTitle>
        <AlertDescription>Ta copie a bien été enregistrée.</AlertDescription>
      </Alert>

      {result.score != null ? (
        <Card>
          <CardHeader>
            <CardTitle>Ton résultat</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <p className="text-3xl font-semibold tabular-nums">
              {result.grade != null ? (
                <>
                  {result.grade}
                  <span className="text-lg text-muted-foreground">/{result.max_grade}</span>
                </>
              ) : (
                <>
                  {result.score}
                  <span className="text-lg text-muted-foreground">/{result.max_score}</span>
                </>
              )}
            </p>
            {result.percentage != null && (
              <p className="text-sm text-muted-foreground">{result.percentage} % de réussite</p>
            )}
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="py-6 text-center text-sm text-muted-foreground">
            Ton résultat sera communiqué par ton professeur.
          </CardContent>
        </Card>
      )}

      {result.correction && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Détail par question</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-1 text-sm">
              {result.correction.per_question.map((q) => (
                <li key={q.question_id} className="flex items-center justify-between gap-3">
                  <span className="truncate font-mono text-xs">{q.question_id}</span>
                  <span className="shrink-0 tabular-nums">
                    {q.points}/{q.max_points}
                  </span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      <Button variant="secondary" className="w-full" onClick={() => location.assign("/eleve")}>
        Retour
      </Button>
    </div>
  );
}

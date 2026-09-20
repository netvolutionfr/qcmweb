"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { ArrowRight, Clock, ListChecks, RotateCcw } from "lucide-react";
import { student, ApiError, type Briefing } from "@/lib/api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export default function Eleve() {
  const router = useRouter();
  const [code, setCode] = useState("");
  const [briefing, setBriefing] = useState<Briefing | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    // Sans cette vérification, l'élève taperait un code pour s'entendre dire
    // qu'il n'est pas connecté.
    student.me().catch((err) => {
      if (err instanceof ApiError && err.status === 401) router.replace("/eleve/connexion");
    });
  }, [router]);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      setBriefing(await student.join(code.trim()));
    } catch (err) {
      setBriefing(null);
      if (err instanceof ApiError && err.status === 401) {
        router.replace("/eleve/connexion");
      } else if (err instanceof ApiError && err.status === 403) {
        setError("Cette évaluation n'est pas destinée à ta classe.");
      } else {
        // Code inconnu et évaluation fermée répondent la même chose côté
        // serveur, pour ne pas confirmer l'existence d'un code.
        setError("Aucune évaluation ouverte avec ce code. Vérifie auprès de ton professeur.");
      }
    } finally {
      setBusy(false);
    }
  }

  async function start() {
    if (!briefing) return;
    setBusy(true);
    try {
      const sitting = await student.sit(briefing.assessment_id);
      router.push(`/eleve/epreuve/${sitting.attempt_id}?a=${briefing.assessment_id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Code de l&apos;évaluation</CardTitle>
          <CardDescription>Le code donné par ton professeur.</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={submit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="code" className="sr-only">
                Code
              </Label>
              <Input
                id="code"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                required
                autoFocus
                autoComplete="off"
                autoCapitalize="characters"
                spellCheck={false}
                placeholder="K7MP4Q"
                className="text-center font-mono text-2xl tracking-[0.3em]"
              />
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <Button type="submit" className="w-full" size="lg" disabled={busy || code.trim() === ""}>
              Valider le code
            </Button>
          </form>
        </CardContent>
      </Card>

      {briefing && (
        <Card>
          <CardHeader>
            <CardTitle>{briefing.subject_title}</CardTitle>
            <CardDescription>{briefing.name}</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {briefing.description && <p className="text-sm">{briefing.description}</p>}

            <ul className="space-y-1 text-sm text-muted-foreground">
              <li className="flex items-center gap-2">
                <ListChecks className="size-4 shrink-0" />
                {briefing.question_count} questions · {briefing.total_points} points
              </li>
              <li className="flex items-center gap-2">
                <Clock className="size-4 shrink-0" />
                {briefing.duration_minutes
                  ? `${briefing.duration_minutes} minutes`
                  : "Sans limite de temps"}
              </li>
              <li className="flex items-center gap-2">
                <RotateCcw className="size-4 shrink-0" />
                {briefing.attempts_left > 0
                  ? `${briefing.attempts_left} tentative${briefing.attempts_left > 1 ? "s" : ""} restante${briefing.attempts_left > 1 ? "s" : ""}`
                  : "Plus de tentative disponible"}
              </li>
            </ul>

            {briefing.resumable_attempt && (
              <Alert>
                <AlertDescription>
                  Tu as une copie commencée : tu vas la retrouver là où tu t&apos;es arrêté.
                </AlertDescription>
              </Alert>
            )}

            <Button
              onClick={start}
              className="w-full"
              size="lg"
              disabled={busy || (briefing.attempts_left <= 0 && !briefing.resumable_attempt)}
            >
              {briefing.resumable_attempt ? "Reprendre" : "Démarrer"}
              <ArrowRight className="size-4" />
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

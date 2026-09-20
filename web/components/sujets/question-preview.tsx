import { Check } from "lucide-react";
import { Markdown } from "@/components/markdown";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import type { Choice, Question } from "@/lib/api";

export type PreviewMode = "eleve" | "corrige";

/**
 * Rendu d'une question pour la relecture.
 *
 * En mode `eleve`, bonnes réponses et explications sont absentes de l'affichage
 * — l'enseignant doit voir exactement ce que verra sa classe (SPEC §5).
 *
 * Ce composant est un outil de relecture, **pas** le composant du parcours
 * élève. Pendant une évaluation, c'est le serveur qui retire `correct` et
 * `explanation` avant l'envoi (SPEC §12) : masquer à l'affichage ne protège de
 * rien, il suffirait d'ouvrir les outils de développement.
 */
export function QuestionPreview({
  question,
  index,
  mode,
}: {
  question: Question;
  index: number;
  mode: PreviewMode;
}) {
  const corrected = mode === "corrige";
  const points = question.points ?? 1;

  const choices: Choice[] =
    question.type === "true_false"
      ? [
          { id: "true", text: "Vrai", correct: question.answer },
          { id: "false", text: "Faux", correct: !question.answer },
        ]
      : question.choices;

  const multiple = question.type === "multiple_choice";

  return (
    <li className="rounded-lg border p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 flex-1">
          <p className="text-xs text-muted-foreground">Question {index + 1}</p>
          <Markdown className="mt-1">{question.prompt}</Markdown>
        </div>
        <Badge variant="secondary" className="shrink-0">
          {points} pt{points > 1 ? "s" : ""}
        </Badge>
      </div>

      {multiple && (
        <p className="mt-1 text-xs text-muted-foreground">Plusieurs réponses possibles.</p>
      )}

      <ul className="mt-3 space-y-2">
        {choices.map((choice) => {
          const right = corrected && choice.correct;
          return (
            <li
              key={choice.id}
              className={cn(
                "flex items-start gap-3 rounded-md border px-3 py-2",
                right && "border-emerald-600/40 bg-emerald-500/10",
              )}
            >
              <span
                aria-hidden
                className={cn(
                  "mt-0.5 flex size-4 shrink-0 items-center justify-center border",
                  multiple ? "rounded-[3px]" : "rounded-full",
                  right ? "border-emerald-600 bg-emerald-600 text-white" : "bg-background",
                )}
              >
                {right && <Check className="size-3" strokeWidth={3} />}
              </span>
              <Markdown className="min-w-0 flex-1 [&_p]:my-0">{choice.text}</Markdown>
              {right && <span className="sr-only">Bonne réponse</span>}
            </li>
          );
        })}
      </ul>

      {corrected && question.explanation && (
        <div className="mt-3 rounded-md border-l-2 border-muted-foreground/30 bg-muted/40 py-2 pl-3">
          <p className="text-xs font-medium text-muted-foreground">Explication</p>
          <Markdown className="mt-1">{question.explanation}</Markdown>
        </div>
      )}

      {corrected && question.objectives && question.objectives.length > 0 && (
        <ul className="mt-3 flex flex-wrap gap-1">
          {question.objectives.map((objective) => (
            <li key={objective}>
              <Badge variant="outline" className="font-normal">
                {objective}
              </Badge>
            </li>
          ))}
        </ul>
      )}

      {corrected && question.type === "multiple_choice" && (
        <p className="mt-3 text-xs text-muted-foreground">
          Notation : {question.scoring?.mode === "partial" ? "partielle" : "exacte"}
        </p>
      )}
    </li>
  );
}

import { Markdown } from "@/components/markdown";
import { ChoiceMark, QuestionFrame } from "@/components/question-frame";
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
  total,
  mode,
}: {
  question: Question;
  index: number;
  total: number;
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
    <li>
      <QuestionFrame
        index={index}
        total={total}
        points={points}
        prompt={question.prompt}
        hint={multiple ? "Plusieurs réponses possibles." : undefined}
      >
        <ul className="space-y-2.5">
          {choices.map((choice) => {
            const right = corrected && choice.correct;
            return (
              <li
                key={choice.id}
                className={cn(
                  "flex items-center gap-3 rounded-md border px-3 py-3",
                  right && "border-emerald-600/40 bg-emerald-500/10",
                )}
              >
                <ChoiceMark multiple={multiple} state={right ? "correct" : "off"} />
                <Markdown className="min-w-0 flex-1">{choice.text}</Markdown>
                {right && <span className="sr-only">Bonne réponse</span>}
              </li>
            );
          })}
        </ul>

        {corrected && question.explanation && (
          <div className="mt-4 rounded-md border-l-2 border-muted-foreground/30 bg-muted/40 py-2 pl-3">
            <p className="text-xs font-medium text-muted-foreground">Explication</p>
            <Markdown className="mt-1">{question.explanation}</Markdown>
          </div>
        )}

        {corrected && question.objectives && question.objectives.length > 0 && (
          <ul className="mt-4 flex flex-wrap gap-1">
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
          <p className="mt-4 text-xs text-muted-foreground">
            Notation : {question.scoring?.mode === "partial" ? "partielle" : "exacte"}
          </p>
        )}
      </QuestionFrame>
    </li>
  );
}

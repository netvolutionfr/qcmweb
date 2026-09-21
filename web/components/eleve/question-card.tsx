"use client";

import { Markdown } from "@/components/markdown";
import { ChoiceMark, QuestionFrame } from "@/components/question-frame";
import { cn } from "@/lib/utils";
import type { ExamQuestion } from "@/lib/api";

/**
 * Une question pendant l'épreuve.
 *
 * Le type reçu du serveur ne comporte aucune bonne réponse : il n'y a donc
 * rien à masquer ici, et rien à trouver dans les outils de développement
 * (ADR-0012).
 *
 * Les zones cliquables couvrent la proposition entière : sur un téléphone, une
 * case à cocher de seize pixels est une source d'erreurs.
 */
export function QuestionCard({
  question,
  index,
  total,
  selected,
  onChange,
  disabled,
}: {
  question: ExamQuestion;
  index: number;
  total: number;
  selected: string[];
  onChange: (choices: string[]) => void;
  disabled: boolean;
}) {
  function toggle(id: string) {
    if (disabled) return;
    if (question.multiple) {
      onChange(selected.includes(id) ? selected.filter((c) => c !== id) : [...selected, id]);
    } else {
      // Recliquer sur la réponse déjà choisie la retire : un élève doit pouvoir
      // revenir à une question sans réponse s'il préfère ne pas se prononcer.
      onChange(selected.includes(id) ? [] : [id]);
    }
  }

  return (
    <QuestionFrame
      index={index}
      total={total}
      points={question.points}
      prompt={question.prompt}
      hint={question.multiple ? "Plusieurs réponses possibles." : undefined}
    >
      <ul className="space-y-2.5">
        {question.choices.map((choice) => {
          const checked = selected.includes(choice.id);
          return (
            <li key={choice.id}>
              <button
                type="button"
                onClick={() => toggle(choice.id)}
                disabled={disabled}
                aria-pressed={checked}
                className={cn(
                  "flex w-full items-center gap-3 rounded-md border px-3 py-3 text-left transition-colors",
                  checked ? "border-foreground/40 bg-accent" : "hover:bg-accent/50",
                  disabled && "opacity-60",
                )}
              >
                <ChoiceMark multiple={question.multiple} state={checked ? "on" : "off"} />
                <Markdown className="min-w-0 flex-1">{choice.text}</Markdown>
              </button>
            </li>
          );
        })}
      </ul>
    </QuestionFrame>
  );
}

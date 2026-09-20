"use client";

import { Check } from "lucide-react";
import { Markdown } from "@/components/markdown";
import { Badge } from "@/components/ui/badge";
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
    <section className="rounded-lg border p-4">
      <div className="flex items-start justify-between gap-3">
        <p className="text-xs text-muted-foreground">
          Question {index + 1} sur {total}
        </p>
        <Badge variant="secondary" className="shrink-0">
          {question.points} pt{question.points > 1 ? "s" : ""}
        </Badge>
      </div>

      <Markdown className="mt-2">{question.prompt}</Markdown>

      {question.multiple && (
        <p className="mt-1 text-xs text-muted-foreground">Plusieurs réponses possibles.</p>
      )}

      <ul className="mt-3 space-y-2">
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
                  "flex w-full items-start gap-3 rounded-md border px-3 py-3 text-left transition-colors",
                  checked ? "border-foreground/40 bg-accent" : "hover:bg-accent/50",
                  disabled && "opacity-60",
                )}
              >
                <span
                  aria-hidden
                  className={cn(
                    "mt-0.5 flex size-5 shrink-0 items-center justify-center border",
                    question.multiple ? "rounded-[4px]" : "rounded-full",
                    checked ? "border-foreground bg-foreground text-background" : "bg-background",
                  )}
                >
                  {checked && <Check className="size-3.5" strokeWidth={3} />}
                </span>
                <Markdown className="min-w-0 flex-1 [&_p]:my-0">{choice.text}</Markdown>
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

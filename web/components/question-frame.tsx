import { Check } from "lucide-react";
import { Markdown } from "@/components/markdown";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

/**
 * Cadre commun à une question, en relecture comme en épreuve.
 *
 * La prévisualisation doit montrer exactement ce que verra la classe
 * (SPEC §5) : les deux vues partagent donc ce cadre plutôt que d'en avoir chacune
 * une copie qui finirait par diverger.
 *
 * L'énoncé est la chose à lire en premier : il est posé sur un bandeau, plus
 * grand et en gras, séparé des propositions.
 */
export function QuestionFrame({
  index,
  total,
  points,
  prompt,
  hint,
  children,
}: {
  index: number;
  total: number;
  points: number;
  prompt: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="overflow-hidden rounded-xl border bg-card text-card-foreground shadow-sm">
      <header className="border-b bg-muted/60 px-4 py-4 sm:px-5 sm:py-5">
        <div className="flex items-center justify-between gap-3">
          <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Question {index + 1}
            <span className="font-normal normal-case tracking-normal"> sur {total}</span>
          </p>
          <Badge variant="secondary" className="shrink-0">
            {points} pt{points > 1 ? "s" : ""}
          </Badge>
        </div>

        <Markdown className="mt-3 text-lg font-semibold leading-snug sm:text-xl [&_code]:font-medium [&_pre]:text-sm [&_pre]:font-normal [&_pre]:leading-normal">
          {prompt}
        </Markdown>

        {hint && <p className="mt-3 text-sm text-muted-foreground">{hint}</p>}
      </header>

      <div className="p-4 sm:p-5">{children}</div>
    </section>
  );
}

/**
 * Case ou pastille d'une proposition.
 *
 * Partagée par les deux vues : c'est ce qui garantit qu'elle a la même taille
 * et le même alignement partout. Elle se centre sur la ligne, les propositions
 * étant alignées avec `items-center`.
 */
export function ChoiceMark({
  multiple,
  state,
}: {
  multiple: boolean;
  state: "off" | "on" | "correct";
}) {
  return (
    <span
      aria-hidden
      className={cn(
        "flex size-5 shrink-0 items-center justify-center border",
        multiple ? "rounded-[4px]" : "rounded-full",
        state === "off" && "bg-background",
        state === "on" && "border-foreground bg-foreground text-background",
        state === "correct" && "border-emerald-600 bg-emerald-600 text-white",
      )}
    >
      {state !== "off" && <Check className="size-3.5" strokeWidth={3} />}
    </span>
  );
}

import Link from "next/link";
import { GraduationCap, PencilRuler } from "lucide-react";

export default function Home() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-sm flex-col justify-center gap-6 px-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">QCMWeb</h1>
        <p className="mt-1 text-sm text-muted-foreground">Évaluation par questionnaire.</p>
      </div>

      <div className="grid gap-3">
        <Link
          href="/eleve"
          className="flex items-center gap-3 rounded-lg border p-4 transition-colors hover:bg-accent"
        >
          <GraduationCap className="size-5 shrink-0 text-muted-foreground" />
          <div>
            <p className="font-medium">Je passe une évaluation</p>
            <p className="text-xs text-muted-foreground">Avec le billet remis en classe</p>
          </div>
        </Link>

        <Link
          href="/sujets"
          className="flex items-center gap-3 rounded-lg border p-4 transition-colors hover:bg-accent"
        >
          <PencilRuler className="size-5 shrink-0 text-muted-foreground" />
          <div>
            <p className="font-medium">Espace enseignant</p>
            <p className="text-xs text-muted-foreground">Sujets, évaluations, résultats</p>
          </div>
        </Link>
      </div>
    </main>
  );
}

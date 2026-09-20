import { Badge } from "@/components/ui/badge";

/**
 * État **et** disponibilité, jamais l'un sans l'autre.
 *
 * Une évaluation peut être ouverte mais hors de sa fenêtre planifiée : afficher
 * « ouverte » seule laisserait croire qu'elle est accessible, alors que les
 * élèves se heurteraient à un refus (ADR-0011).
 */
export function StateBadge({ state, available }: { state: string; available: boolean }) {
  if (available) return <Badge>en cours</Badge>;

  if (state === "OPEN") {
    return (
      <Badge variant="outline" className="text-amber-700 dark:text-amber-500">
        ouverte, hors fenêtre
      </Badge>
    );
  }
  return (
    <Badge variant="secondary">{state === "CLOSED" ? "fermée" : "non ouverte"}</Badge>
  );
}

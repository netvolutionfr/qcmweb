import type { Slip } from "@/lib/roster";

/**
 * Aperçu à l'écran de la planche de billets.
 *
 * La planche imprimable est produite par `lib/billets-pdf.ts`, composée au
 * millimètre. Cet aperçu sert à vérifier les données avant téléchargement, pas
 * à être imprimé : `items-start` empêche les lignes de s'étirer, ce qui
 * donnerait des billets de hauteur variable.
 */
export function BilletSheet({
  slips,
  group,
}: {
  slips: Slip[];
  group?: { label: string; school_year: string };
}) {
  return (
    <div className="zone-impression grid grid-cols-1 items-start gap-3 sm:grid-cols-2 print:grid-cols-2">
      {slips.map((slip) => (
        <article key={slip.id} className="billet rounded-lg border p-4">
          <p className="font-medium">
            {slip.firstName} {slip.lastName}
          </p>
          {group && (
            <p className="text-xs text-muted-foreground">
              {group.label} — {group.school_year}
            </p>
          )}

          <dl className="mt-3 space-y-1 font-mono text-sm">
            <div className="flex gap-3">
              <dt className="w-14 shrink-0 font-sans text-xs text-muted-foreground">jeton</dt>
              <dd className="tracking-wide">{slip.token}</dd>
            </div>
            <div className="flex gap-3">
              <dt className="w-14 shrink-0 font-sans text-xs text-muted-foreground">secret</dt>
              <dd className="tracking-wide">{slip.secret}</dd>
            </div>
          </dl>
        </article>
      ))}
    </div>
  );
}

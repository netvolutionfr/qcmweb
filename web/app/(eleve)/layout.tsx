/**
 * Coque du parcours élève : volontairement nue.
 *
 * Ni navigation, ni lien vers l'espace enseignant. Un élève en épreuve n'a
 * qu'une chose à faire, et chaque élément d'interface superflu est une occasion
 * de se perdre — sur un téléphone plus encore.
 */
export default function ExamLayout({ children }: { children: React.ReactNode }) {
  return <div className="mx-auto max-w-2xl px-4 py-6 sm:py-10">{children}</div>;
}

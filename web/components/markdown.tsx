import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeHighlight from "rehype-highlight";
import { cn } from "@/lib/utils";

/**
 * Rendu Markdown des énoncés et des propositions (SPEC §6).
 *
 * `react-markdown` échappe le HTML brut par défaut, et `rehype-raw` n'est
 * délibérément pas activé : un sujet peut être rédigé par un agent à partir
 * d'un document quelconque, il n'y a aucune raison de lui laisser injecter du
 * balisage dans l'écran de relecture.
 */
export function Markdown({ children, className }: { children: string; className?: string }) {
  return (
    <div
      // `cn` et non une simple concaténation : l'appelant doit pouvoir écraser
      // une classe de base (`[&_p]:my-0` contre `[&_p]:my-2`). En concaténant, le
      // gagnant dépend de l'ordre dans la feuille de style, pas de celui écrit ici.
      className={cn(
        "max-w-none [&_p]:my-2 [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5",
        "[&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5",
        "[&_code]:rounded [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.9em]",
        "[&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:border [&_pre]:bg-muted [&_pre]:p-3",
        "[&_pre_code]:bg-transparent [&_pre_code]:p-0",
        "[&_table]:my-3 [&_table]:w-full [&_th]:border-b [&_th]:text-left [&_th]:p-1 [&_td]:p-1",
        // Le premier et le dernier bloc ne poussent pas leur marge hors du
        // conteneur : sans cela, le texte d'une proposition n'est jamais
        // centré verticalement dans sa ligne.
        "[&>:first-child]:mt-0 [&>:last-child]:mb-0",
        className,
      )}
    >
      <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeHighlight]}>
        {children}
      </ReactMarkdown>
    </div>
  );
}

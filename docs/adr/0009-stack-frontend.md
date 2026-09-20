# ADR-0009 — Stack frontend : Next.js, Tailwind, shadcn/ui, thème système

**Statut** : Acceptée — 2026-09-20

## Contexte

Le front enseignant se limite à quatre écrans
([ADR-0003](0003-administration-par-facade-mcp.md)), auxquels s'ajoute le
parcours élève. Ce dernier s'exécutera majoritairement sur **téléphone**, en
salle, sur des appareils et des connexions hétérogènes.

## Décision

**Next.js (App Router) + TypeScript + Tailwind v4 + shadcn/ui**, polices
**Geist Sans** et **Geist Mono**, icônes **lucide**.

### shadcn/ui plutôt qu'une bibliothèque de composants

Les composants sont **copiés dans le dépôt**, pas importés d'un paquet. Ils sont
donc lisibles, modifiables et versionnés avec le reste. Pour une application qui
vivra des années entre deux périodes d'activité, c'est préférable à une
dépendance dont les versions majeures imposeraient des migrations.

### Geist Mono pour les codes

La variante monospace n'est pas décorative. Jetons et secrets sont recopiés
caractère par caractère depuis un billet papier : une chasse fixe évite de
confondre des glyphes voisins, et complète le travail fait sur l'alphabet en
[ADR-0007](0007-format-des-codes.md).

### Thème lié à la préférence système, sans JavaScript

Le variant `dark:` de Tailwind est redéfini sur la media query :

```css
@custom-variant dark (@media (prefers-color-scheme: dark));
```

et les variables de thème vivent sous `@media (prefers-color-scheme: dark)`
plutôt que sous une classe `.dark`.

Conséquence : **aucune dépendance, aucun script, et surtout aucun flash de thème
clair avant l'hydratation** — défaut classique des solutions à base de classe,
qui doivent lire la préférence en JavaScript après le premier rendu.

Le thème du navigateur (`theme-color`) suit la même préférence.

### Mobile d'abord

Les styles sont écrits pour téléphone puis élargis à `sm`. La planche de billets
passe à deux colonnes à partir de `sm`, et **reste à deux colonnes à
l'impression** : la grille d'écran n'a aucune raison de dicter la mise en page
d'une feuille A4.

### Même origine, types générés

Un `rewrites` Next.js proxifie `/api/*` vers le backend, reproduisant en
développement ce que Nginx fait en production
([ADR-0008](0008-deploiement-nginx-docker.md)). Aucun CORS, et le cookie de
session conserve `SameSite=Strict`.

Les types TypeScript sont **générés** depuis l'OpenAPI (`npm run types`), jamais
écrits à la main.

## Conséquences

- Ajouter une bascule claire/sombre manuelle exigerait `next-themes` et le
  retour au variant par classe. C'est un renoncement assumé : la demande était
  de suivre le système.
- La garde de session côté client (`RequireSession`) n'est **pas** un contrôle
  de sécurité — le serveur vérifie l'autorisation à chaque requête. Elle évite
  seulement d'afficher une page vide.
- Les composants `ui/` étant du code du dépôt, leurs mises à jour sont
  manuelles. C'est le prix de leur modifiabilité.

## Alternatives écartées

- **next-themes** — la solution habituelle, mais elle apporte une dépendance et
  un script d'anti-flash pour une fonctionnalité que le CSS natif couvre
  entièrement dès lors qu'aucune bascule manuelle n'est demandée.
- **Material UI / Mantine** — composants complets mais opaques, et un poids
  injustifié pour cinq écrans.
- **Un front sans framework** — il faudrait réécrire le routage, la gestion de
  formulaires et le rendu. Next.js est déjà retenu par la SPEC §19.

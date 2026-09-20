"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { api, ApiError } from "@/lib/api";

/**
 * Garde de session côté client.
 *
 * Ce n'est **pas** un contrôle de sécurité : l'autorisation est systématiquement
 * vérifiée par le serveur, qui répond 401 quoi qu'affiche le navigateur
 * (SPEC §17). Il s'agit seulement d'éviter d'afficher une page vide à qui
 * n'est pas connecté.
 */
export function RequireSession({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const [state, setState] = useState<"checking" | "ok">("checking");

  useEffect(() => {
    api
      .me()
      .then(() => setState("ok"))
      .catch((err) => {
        if (err instanceof ApiError && err.status === 401) router.replace("/connexion");
        else setState("ok"); // panne réseau : laisser l'écran se débrouiller
      });
  }, [router]);

  if (state === "checking") {
    return (
      <div className="flex min-h-dvh items-center justify-center">
        <p className="text-sm text-muted-foreground">Chargement…</p>
      </div>
    );
  }
  return <>{children}</>;
}

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { api, ApiError } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Alert, AlertDescription } from "@/components/ui/alert";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export default function Connexion() {
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    setBusy(true);
    setError(null);
    try {
      await api.login(String(form.get("username")), String(form.get("password")));
      router.replace("/billets");
    } catch (err) {
      // Un message unique pour les deux cas : distinguer « identifiant inconnu »
      // de « mot de passe faux » confirmerait l'identifiant à un inconnu.
      setError(
        err instanceof ApiError && err.status === 429
          ? "Trop de tentatives. Réessayez dans un quart d'heure."
          : "Identifiant ou mot de passe incorrect.",
      );
      setBusy(false);
    }
  }

  return (
    <main className="flex min-h-dvh items-center justify-center p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>QCMWeb</CardTitle>
          <CardDescription>Espace enseignant</CardDescription>
        </CardHeader>

        <CardContent>
          <form onSubmit={submit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="username">Identifiant</Label>
              <Input id="username" name="username" autoComplete="username" required autoFocus />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">Mot de passe</Label>
              <Input
                id="password"
                name="password"
                type="password"
                autoComplete="current-password"
                required
              />
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <Button type="submit" className="w-full" disabled={busy}>
              {busy ? "Connexion…" : "Se connecter"}
            </Button>
          </form>

          <p className="mt-6 text-xs text-muted-foreground">
            Il n&apos;existe pas de récupération de mot de passe : elle supposerait une adresse
            électronique, que cette application ne stocke pas.
          </p>
        </CardContent>
      </Card>
    </main>
  );
}

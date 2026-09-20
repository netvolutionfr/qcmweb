"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { student, ApiError } from "@/lib/api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

export default function ConnexionEleve() {
  const router = useRouter();
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = new FormData(event.currentTarget);
    setBusy(true);
    setError(null);
    try {
      await student.login(String(form.get("token")), String(form.get("secret")));
      router.replace("/eleve");
    } catch (err) {
      setError(
        err instanceof ApiError && err.status === 429
          ? "Trop d'essais. Patiente un quart d'heure ou préviens ton professeur."
          : "Jeton ou secret incorrect. Vérifie ton billet.",
      );
      setBusy(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Connexion</CardTitle>
        <CardDescription>Recopie les deux codes inscrits sur ton billet.</CardDescription>
      </CardHeader>

      <CardContent>
        <form onSubmit={submit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="token">Jeton</Label>
            <Input
              id="token"
              name="token"
              required
              autoFocus
              autoComplete="off"
              autoCapitalize="characters"
              spellCheck={false}
              placeholder="ZQ93-GSKN"
              className="font-mono text-lg tracking-widest"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="secret">Secret</Label>
            <Input
              id="secret"
              name="secret"
              required
              autoComplete="off"
              autoCapitalize="characters"
              spellCheck={false}
              placeholder="PMQX4-7ME4E"
              className="font-mono text-lg tracking-widest"
            />
          </div>

          {error && (
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          <Button type="submit" className="w-full" size="lg" disabled={busy}>
            {busy ? "Connexion…" : "Se connecter"}
          </Button>

          <p className="text-xs text-muted-foreground">
            Les tirets et les majuscules n&apos;ont pas d&apos;importance.
          </p>
        </form>
      </CardContent>
    </Card>
  );
}

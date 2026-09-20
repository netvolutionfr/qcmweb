"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { ClipboardList, TriangleAlert } from "lucide-react";
import { api, type Assessment } from "@/lib/api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { StateBadge } from "@/components/evaluations/state-badge";

export default function Evaluations() {
  const [items, setItems] = useState<Assessment[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .assessments()
      .then(setItems)
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Évaluations</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Ouvrir, fermer et consulter les résultats.
        </p>
      </div>

      {error && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {items?.length === 0 && (
        <Card>
          <CardContent className="py-10 text-center text-sm text-muted-foreground">
            Aucune évaluation. Demandez à votre agent d&apos;en créer une sur un sujet validé.
          </CardContent>
        </Card>
      )}

      <ul className="space-y-2">
        {items?.map((item) => (
          <li key={item.id}>
            <Link
              href={`/evaluations/${item.id}`}
              className="flex flex-wrap items-center gap-3 rounded-lg border p-4 transition-colors hover:bg-accent"
            >
              <ClipboardList className="size-5 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{item.name}</p>
                <p className="truncate text-xs text-muted-foreground">
                  {item.group_label} · {item.subject_title} v{item.subject_version}
                </p>
              </div>
              <Badge variant="outline" className="font-mono tracking-wider">
                {item.code}
              </Badge>
              <StateBadge state={item.state} available={item.available} />
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}

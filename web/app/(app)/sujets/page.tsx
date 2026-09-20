"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Archive, FileText, TriangleAlert } from "lucide-react";
import { api, type Subject } from "@/lib/api";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";

export default function Sujets() {
  const [subjects, setSubjects] = useState<Subject[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api
      .subjects()
      .then(setSubjects)
      .catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Sujets</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Les sujets sont déposés par un agent et relus ici avant publication.
        </p>
      </div>

      {error && (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {subjects?.length === 0 && (
        <Card>
          <CardContent className="py-10 text-center text-sm text-muted-foreground">
            Aucun sujet pour l&apos;instant. Demandez à votre agent d&apos;en déposer un.
          </CardContent>
        </Card>
      )}

      <ul className="space-y-2">
        {subjects?.map((subject) => (
          <li key={subject.id}>
            <Link
              href={`/sujets/${subject.id}/versions/${subject.latest_version}`}
              className="flex items-center gap-3 rounded-lg border p-4 transition-colors hover:bg-accent"
            >
              <FileText className="size-5 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">{subject.title}</p>
                <p className="text-xs text-muted-foreground">
                  version {subject.latest_version}
                </p>
              </div>
              {subject.archived && (
                <Badge variant="outline" className="gap-1">
                  <Archive className="size-3" />
                  archivé
                </Badge>
              )}
              <Badge variant={subject.latest_status === "VALIDATED" ? "default" : "secondary"}>
                {subject.latest_status === "VALIDATED" ? "validé" : "brouillon"}
              </Badge>
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}

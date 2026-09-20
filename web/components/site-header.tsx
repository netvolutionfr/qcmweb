"use client";

import { useRouter } from "next/navigation";
import { LogOut } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";

export function SiteHeader() {
  const router = useRouter();

  return (
    <header className="sticky top-0 z-10 border-b bg-background/80 backdrop-blur print:hidden">
      <div className="mx-auto flex h-14 max-w-4xl items-center justify-between gap-4 px-4">
        <div className="flex min-w-0 items-baseline gap-2">
          <span className="font-semibold">QCMWeb</span>
          <span className="truncate text-sm text-muted-foreground">Espace enseignant</span>
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => api.logout().finally(() => router.replace("/connexion"))}
        >
          <LogOut className="size-4" />
          {/* Sur mobile, l'icône se suffit : le libellé mangerait la barre. */}
          <span className="sr-only sm:not-sr-only">Se déconnecter</span>
        </Button>
      </div>
    </header>
  );
}

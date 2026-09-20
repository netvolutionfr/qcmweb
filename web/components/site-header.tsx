"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { LogOut } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const LINKS = [
  { href: "/sujets", label: "Sujets" },
  { href: "/billets", label: "Billets" },
];

export function SiteHeader() {
  const router = useRouter();
  const pathname = usePathname();

  return (
    <header className="sticky top-0 z-10 border-b bg-background/80 backdrop-blur print:hidden">
      <div className="mx-auto flex h-14 max-w-4xl items-center gap-4 px-4">
        <Link href="/sujets" className="font-semibold">
          QCMWeb
        </Link>

        <nav className="flex flex-1 items-center gap-1">
          {LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                "rounded-md px-3 py-1.5 text-sm transition-colors",
                pathname.startsWith(link.href)
                  ? "bg-accent font-medium text-accent-foreground"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {link.label}
            </Link>
          ))}
        </nav>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => api.logout().finally(() => router.replace("/connexion"))}
        >
          <LogOut className="size-4" />
          {/* Sur mobile l'icône se suffit : le libellé mangerait la barre. */}
          <span className="sr-only sm:not-sr-only">Se déconnecter</span>
        </Button>
      </div>
    </header>
  );
}

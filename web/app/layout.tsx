import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import { cn } from "@/lib/utils";
import "./globals.css";

// Polices par défaut de shadcn/ui. La variante monospace n'est pas décorative :
// jetons et secrets sont recopiés caractère par caractère depuis un billet
// papier, et une chasse fixe évite de confondre les glyphes voisins.
const sans = Geist({ subsets: ["latin"], variable: "--font-sans" });
const mono = Geist_Mono({ subsets: ["latin"], variable: "--font-mono" });

export const metadata: Metadata = {
  title: "QCMWeb",
  description: "Plateforme d'évaluation par QCM",
};

export const viewport: Viewport = {
  // Le thème du navigateur suit la préférence système, comme l'interface.
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#ffffff" },
    { media: "(prefers-color-scheme: dark)", color: "#0a0a0a" },
  ],
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="fr" className={cn(sans.variable, mono.variable)}>
      <body className="min-h-dvh font-sans antialiased">{children}</body>
    </html>
  );
}

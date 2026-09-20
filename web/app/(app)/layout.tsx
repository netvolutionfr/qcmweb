import { RequireSession } from "@/components/require-session";
import { SiteHeader } from "@/components/site-header";

export default function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <RequireSession>
      <SiteHeader />
      <main className="mx-auto max-w-4xl px-4 py-6 sm:py-10">{children}</main>
    </RequireSession>
  );
}

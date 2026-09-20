"use client";

import { useEffect, useState } from "react";
import { Clock } from "lucide-react";
import { remainingSeconds } from "@/lib/exam-storage";
import { cn } from "@/lib/utils";

/**
 * Compte à rebours calé sur l'heure du serveur.
 *
 * L'horloge d'un poste de salle informatique est régulièrement fausse, parfois
 * de plusieurs minutes : afficher le temps restant d'après `Date.now()` seul
 * donnerait une durée erronée, et le serveur refuserait les réponses bien avant
 * la fin annoncée.
 */
export function Countdown({
  deadline,
  offset,
  onExpire,
}: {
  deadline: string | null;
  offset: number;
  onExpire: () => void;
}) {
  const [left, setLeft] = useState(() => remainingSeconds(deadline, offset));

  useEffect(() => {
    if (!deadline) return;
    const tick = () => {
      const value = remainingSeconds(deadline, offset);
      setLeft(value);
      if (value === 0) onExpire();
    };
    tick();
    const timer = setInterval(tick, 1000);
    return () => clearInterval(timer);
  }, [deadline, offset, onExpire]);

  if (left == null) return null;

  const minutes = Math.floor(left / 60);
  const urgent = left <= 60;
  const warning = left <= 300;

  return (
    <div
      role="timer"
      aria-live={urgent ? "assertive" : "off"}
      className={cn(
        "flex items-center gap-1.5 rounded-md border px-2 py-1 text-sm tabular-nums",
        urgent && "border-destructive text-destructive",
        warning && !urgent && "border-amber-500 text-amber-700 dark:text-amber-500",
      )}
    >
      <Clock className="size-4 shrink-0" />
      {minutes}:{String(left % 60).padStart(2, "0")}
    </div>
  );
}

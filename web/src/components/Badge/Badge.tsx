import type { ReactNode } from "react";
import styles from "./Badge.module.css";

export type BadgeTone = "verified" | "deal" | "neutral" | "pending" | "rejected";

interface BadgeProps {
  tone?: BadgeTone;
  size?: "default" | "large";
  children: ReactNode;
  className?: string;
}

/** Pill label. verified: accent-soft, deal: accent, neutral: surface, pending: warn, rejected: danger. */
export function Badge({ tone = "neutral", size = "default", children, className }: BadgeProps) {
  return (
    <span className={[styles.badge, styles[tone], size === "large" && styles.large, className].filter(Boolean).join(" ")}>
      {children}
    </span>
  );
}

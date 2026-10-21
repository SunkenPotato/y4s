import type { Me } from "@/lib/api/types";

/** Where to send someone after they sign in or verify, given their status. */
export function nextStepFor(me: Me, fallback = "/"): string {
  switch (me.status) {
    case "verified":
      return fallback;
    case "pending_email":
    case "pending_school":
    case "rejected":
      return "/signup";
    default:
      return "/me";
  }
}

/** Only allow same-site relative paths as ?next= targets. */
export function safeNext(next: string | string[] | undefined | null, fallback = "/"): string {
  const v = Array.isArray(next) ? next[0] : next;
  return v && v.startsWith("/") && !v.startsWith("//") ? v : fallback;
}

export const loginHref = (next: string) => `/login?next=${encodeURIComponent(next)}`;

import type { Me } from "@/lib/api/types";
import { monthYear } from "@/lib/format";

/** Where to send someone after they sign in or verify, given their status. */
export function nextStepFor(me: Me, fallback = "/"): string {
  switch (me.status) {
    case "verified":
    case "expired":
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

/** Shown wherever an expired user would post, and for access_expired errors. */
export const accessEndedMessage = (accessUntil: string) =>
  `Your Corner access ended ${monthYear(accessUntil)}. Verify a new school email to keep posting.`;

/** Who's looking, for UI that depends on posting rights. */
export type Viewer = "signed_out" | "unverified" | "verified" | "expired";

export function viewerOf(me: Me | null): Viewer {
  if (!me) return "signed_out";
  if (me.status === "verified") return "verified";
  if (me.status === "expired") return "expired";
  return "unverified";
}

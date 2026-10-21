import type { Author, Category, Place, PriceLevel, SchoolKind, SchoolSummary } from "@/lib/api/types";

export const CATEGORY_LABEL: Record<Category, string> = {
  food: "Food",
  coffee: "Coffee",
  study: "Study spots",
  gym: "Gyms",
  services: "Services",
  nightlife: "Nightlife",
  groceries: "Groceries",
  other: "Other",
};

/** Singular label used in meta lines when a place has no subcategory. */
export const CATEGORY_SINGULAR: Record<Category, string> = {
  food: "Food",
  coffee: "Coffee",
  study: "Study spot",
  gym: "Gym",
  services: "Services",
  nightlife: "Nightlife",
  groceries: "Groceries",
  other: "Other",
};

/** The feed and search filter chips, in mockup order. "campus" maps to kind=facility. */
export const FILTERS = [
  { value: "all", label: "Everything" },
  { value: "food", label: "Food" },
  { value: "coffee", label: "Coffee" },
  { value: "study", label: "Study spots" },
  { value: "campus", label: "On campus" },
  { value: "gym", label: "Gyms" },
  { value: "services", label: "Services" },
] as const;

export const price = (level: PriceLevel | null | undefined) => (level ? "$".repeat(level) : null);

export const distance = (mi: number | null | undefined) =>
  mi === null || mi === undefined ? null : `${mi < 10 ? mi.toFixed(1) : Math.round(mi)} mi`;

export const count = (n: number) => n.toLocaleString("en-US");

export const plural = (n: number, one: string, many = `${one}s`) => `${count(n)} ${n === 1 ? one : many}`;

export const rating = (avg: number | null | undefined) => (avg === null || avg === undefined ? null : avg.toFixed(1));

/** "Ramen · $ · 0.3 mi" or "Study spot · On campus". */
export function placeMeta(p: Pick<Place, "category" | "subcategory" | "price_level" | "distance_mi" | "kind">): string {
  const parts = [p.subcategory || CATEGORY_SINGULAR[p.category], price(p.price_level)];
  if (p.kind === "facility") parts.push("On campus");
  else parts.push(distance(p.distance_mi));
  return parts.filter(Boolean).join(" · ");
}

export const schoolKind = (kind: SchoolKind) => (kind === "high_school" ? "High school" : "College");

export const schoolPlace = (s: Pick<SchoolSummary, "city" | "state">) => `${s.city}, ${s.state}`;

/** "MR" from "Maya R." */
export function initials(name: string): string {
  return (
    name
      .split(/\s+/)
      .map((w) => w.replace(/[^\p{L}]/gu, "").charAt(0))
      .filter(Boolean)
      .slice(0, 2)
      .join("")
      .toUpperCase() || "?"
  );
}

export const authorLine = (a: Author) => `${a.display_name}, ${a.school_name}`;

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_LONG = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

/** "Oct 14", or "Oct 14, 2025" when it isn't the given current year. Uses UTC so server and client agree. */
export function shortDate(iso: string, currentYear?: number): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const base = `${MONTHS[d.getUTCMonth()]} ${d.getUTCDate()}`;
  return currentYear === undefined || d.getUTCFullYear() === currentYear ? base : `${base}, ${d.getUTCFullYear()}`;
}

/** "2026-10" to "October 2026". */
export function monthLabel(ym: string): string {
  const [y, m] = ym.split("-").map(Number);
  if (!y || !m) return ym;
  return `${MONTHS_LONG[m - 1]} ${y}`;
}

/** The last `n` months as "YYYY-MM", newest first. */
export function recentMonths(now: Date, n = 12): string[] {
  const out: string[] = [];
  const d = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), 1));
  for (let i = 0; i < n; i++) {
    out.push(`${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, "0")}`);
    d.setUTCMonth(d.getUTCMonth() - 1);
  }
  return out;
}

/** "students.lausd.net" to "lausd.net". Naive, but good enough for the admin hint. */
export function baseDomain(domain: string): string {
  const parts = domain.split(".");
  if (parts.length <= 2) return domain;
  const tail = parts.slice(-2).join(".");
  // Keep three labels for short second-level suffixes like k12.ca.us or co.uk.
  return parts[parts.length - 2].length <= 3 && parts.length >= 3 ? parts.slice(-3).join(".") : tail;
}

export const emailDomain = (email: string) => email.split("@")[1] ?? "";

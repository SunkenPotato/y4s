import { CATEGORIES, type Category, type PlaceQuery } from "@/lib/api/types";

/** Filter chip value ("all", a category, or "campus") to a /search URL. */
export function filterHref(value: string, base = "/search", q?: string, extra: Record<string, string> = {}): string {
  const params = new URLSearchParams();
  if (q) params.set("q", q);
  for (const [k, v] of Object.entries(extra)) params.set(k, v);
  if (value === "campus") params.set("kind", "facility");
  else if (value !== "all") params.set("category", value);
  const s = params.toString();
  return s ? `${base}?${s}` : base;
}

export interface SearchState {
  q: string;
  category: Category | null;
  kind: "facility" | null;
  deals: boolean;
  radius: number | null;
}

const first = (v: string | string[] | undefined) => (Array.isArray(v) ? v[0] : v);

export function parseSearchParams(sp: Record<string, string | string[] | undefined>): SearchState {
  const category = first(sp.category);
  const radius = Number(first(sp.radius));
  return {
    q: (first(sp.q) ?? "").trim().slice(0, 100),
    category: category && (CATEGORIES as readonly string[]).includes(category) ? (category as Category) : null,
    kind: first(sp.kind) === "facility" ? "facility" : null,
    deals: first(sp.deals) === "true" || first(sp.deals) === "1",
    radius: radius > 0 && radius <= 10 ? radius : null,
  };
}

/** The chip value that matches the current filters. */
export function filterValue(s: SearchState): string {
  if (s.kind === "facility") return "campus";
  return s.category ?? "all";
}

export function toPlaceQuery(s: SearchState, school: { id: string; lat: number | null; lng: number | null }): PlaceQuery {
  const hasCoords = school.lat !== null && school.lng !== null;
  return {
    q: s.q || undefined,
    category: s.category ?? undefined,
    kind: s.kind ?? undefined,
    deals: s.deals || undefined,
    // A widened search uses near + radius; otherwise the school's own area.
    ...(s.radius && hasCoords ? { near: `${school.lat},${school.lng}`, radius_mi: s.radius } : { school_id: school.id }),
  };
}

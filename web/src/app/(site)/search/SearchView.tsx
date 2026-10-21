"use client";

import { useRouter } from "next/navigation";
import { useMemo, useState, useSyncExternalStore, useTransition } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { Chip } from "@/components/Chip/Chip";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { FilterChips } from "@/components/FilterChips/FilterChips";
import { Aside, Columns, MainColumn, Muted, PageHead, Row, Stack } from "@/components/Layout/Layout";
import { Map as PlaceMap } from "@/components/Map/Map";
import { PlaceTile } from "@/components/PlaceTile/PlaceTile";
import { api, isApiError } from "@/lib/api/client";
import type { Page, Place, School } from "@/lib/api/types";
import { filterHref, filterValue, toPlaceQuery, type SearchState } from "@/lib/filters";
import { CATEGORY_LABEL, FILTERS } from "@/lib/format";
import styles from "./search.module.css";

export type SearchMode = "search" | "deals" | "review";

interface SearchViewProps {
  mode: SearchMode;
  state: SearchState;
  school: School;
  /** First page from the server, or null when that fetch failed. */
  initial: Page<Place> | null;
  /** When the server fetched `initial` (ms since epoch). */
  fetchedAt: number;
  suggestions: Place[];
}

interface Results {
  key: string;
  items: Place[];
  next: string | null;
  fetchedAt: number;
}

const BASE: Record<SearchMode, string> = { search: "/search", deals: "/deals", review: "/review" };

const subscribeClock = (cb: () => void) => {
  const t = window.setInterval(cb, 30_000);
  return () => window.clearInterval(t);
};

export function SearchView({ mode, state, school, initial, fetchedAt, suggestions }: SearchViewProps) {
  const router = useRouter();
  const [pending, startTransition] = useTransition();
  const key = JSON.stringify(state);

  // Keep the last good results so a failed refresh still shows something.
  const [results, setResults] = useState<Results | null>(() =>
    initial ? { key, items: initial.items, next: initial.next_cursor, fetchedAt } : null,
  );
  const [moreError, setMoreError] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);

  // New server data (the URL changed or a refresh finished): adopt it during render.
  const [seen, setSeen] = useState(initial);
  if (initial !== seen) {
    setSeen(initial);
    if (initial) {
      setResults({ key, items: initial.items, next: initial.next_cursor, fetchedAt });
      setMoreError(null);
    }
  }

  const now = useSyncExternalStore(subscribeClock, () => Math.floor(Date.now() / 30_000) * 30_000, () => 0);
  const stale = !initial && results !== null;
  const minutesAgo = results && now ? Math.max(1, Math.round((now - results.fetchedAt) / 60_000)) : 1;

  const base = BASE[mode];
  const chips = FILTERS.map((f) => ({ ...f, href: filterHref(f.value, base, state.q || undefined) }));
  const selected = filterValue(state);
  const items = results?.items ?? [];
  const placeHref = (p: Place) => (mode === "review" ? `/places/${p.id}/review` : `/places/${p.id}`);

  const points = useMemo(
    () => items.map((p) => ({ id: p.id, lat: p.lat, lng: p.lng, label: p.name, href: placeHref(p) })),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [items, mode],
  );

  async function loadMore() {
    if (!results?.next) return;
    setLoadingMore(true);
    setMoreError(null);
    try {
      const page = await api.places({ ...toPlaceQuery(state, school), cursor: results.next });
      setResults({ ...results, items: [...results.items, ...page.items], next: page.next_cursor });
    } catch (e) {
      setMoreError(isApiError(e) ? e.message : "Couldn't load more places. Try again.");
    } finally {
      setLoadingMore(false);
    }
  }

  const retry = () => startTransition(() => router.refresh());

  const what = state.q
    ? `“${state.q}”`
    : state.kind === "facility"
      ? "on-campus places"
      : state.category
        ? CATEGORY_LABEL[state.category].toLowerCase()
        : mode === "deals"
          ? "deals"
          : "places";

  const title =
    mode === "deals"
      ? "Student deals"
      : mode === "review"
        ? "Which place are you reviewing?"
        : state.q
          ? `“${state.q}” near ${school.name}`
          : `Places near ${school.name}`;

  const lede =
    mode === "deals"
      ? `Discounts near ${school.name}. Show your Corner profile before you pay.`
      : mode === "review"
        ? `Search above, or pick a place near ${school.name}.`
        : "Reviewed by verified students.";

  return (
    <div className={styles.screen}>
      <section className={styles.intro}>
        <PageHead title={title} size={mode === "review" ? "title" : "display"}>
          {lede}
        </PageHead>
        <FilterChips label="Categories" options={chips} value={selected} className={styles.chips} />
      </section>

      {stale ? (
        <Alert
          tone="error"
          className={styles.alert}
          action={
            <Button size={36} variant="secondary" onClick={retry} loading={pending} loadingLabel="Retrying…">
              Retry
            </Button>
          }
        >
          Couldn&apos;t refresh. You&apos;re seeing results from {minutesAgo} min ago.
        </Alert>
      ) : null}

      {!results ? (
        <EmptyState
          icon="warning"
          title="Couldn't load places"
          action={
            <Button onClick={retry} loading={pending} loadingLabel="Retrying…">
              Try again
            </Button>
          }
        >
          Corner didn&apos;t answer. Check your connection, then try again.
        </EmptyState>
      ) : items.length === 0 ? (
        <div className={styles.empty}>
          <EmptyState
            title={`No ${what} near ${school.name} yet`}
            action={
              <Button href={state.q ? `/places/new?name=${encodeURIComponent(state.q)}` : "/places/new"}>
                Add a place
              </Button>
            }
          >
            Know one? Add it and be the first to review it.
          </EmptyState>
          <Stack gap={2} className={styles.tryInstead}>
            <Muted small as="h2">
              Try instead
            </Muted>
            <Row gap={2}>
              {selected !== "all" ? <Chip href={filterHref("all", base, state.q || undefined)}>Everything</Chip> : null}
              {suggestions.map((p) => (
                <Chip key={p.id} href={placeHref(p)}>
                  {p.name}
                </Chip>
              ))}
              {!state.radius ? (
                <Chip href={filterHref(selected, base, state.q || undefined, { radius: "3" })}>
                  Search within 3 mi
                </Chip>
              ) : null}
            </Row>
          </Stack>
        </div>
      ) : (
        <Columns gap={6} className={styles.results}>
          <MainColumn>
            <h2 className="visually-hidden">Results</h2>
            <div className={styles.list} aria-busy={pending || undefined}>
              {items.map((p) => (
                <PlaceTile key={p.id} place={p} href={placeHref(p)} />
              ))}
            </div>
            {moreError ? (
              <Alert tone="error" className={styles.alert}>
                {moreError}
              </Alert>
            ) : null}
            {results.next ? (
              <div className={styles.more}>
                <Button variant="secondary" onClick={loadMore} loading={loadingMore} loadingLabel="Loading…">
                  Show more places
                </Button>
              </div>
            ) : null}
          </MainColumn>
          <Aside label="Map">
            <div className={styles.mapWrap}>
              <PlaceMap
                label={`Map of ${what} near ${school.name}`}
                points={points}
                center={school.lat !== null && school.lng !== null ? { lat: school.lat, lng: school.lng } : undefined}
                height={420}
              />
            </div>
          </Aside>
        </Columns>
      )}
    </div>
  );
}

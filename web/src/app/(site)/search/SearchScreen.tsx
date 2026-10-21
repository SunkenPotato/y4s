import { connection } from "next/server";
import { Suspense } from "react";
import { Container, LoadingNote, Placeholder, Stack } from "@/components/Layout/Layout";
import type { Page, Place } from "@/lib/api/types";
import { getServerApi } from "@/lib/api/server";
import { parseSearchParams, toPlaceQuery } from "@/lib/filters";
import { SearchView, type SearchMode } from "./SearchView";

type SP = Record<string, string | string[] | undefined>;

/** Shared by /search, /deals and /review: reads the URL, fetches the first page, hands off to SearchView. */
export function SearchScreen({ searchParams, mode }: { searchParams: Promise<SP>; mode: SearchMode }) {
  return (
    <Container>
      <Suspense fallback={<SearchFallback />}>
        <SearchResults searchParams={searchParams} mode={mode} />
      </Suspense>
    </Container>
  );
}

async function SearchResults({ searchParams, mode }: { searchParams: Promise<SP>; mode: SearchMode }) {
  const state = parseSearchParams(await searchParams);
  if (mode === "deals") state.deals = true;
  const api = await getServerApi();
  const feed = await api.feed();
  const { initial, fetchedAt } = await loadFirstPage(() => api.places(toPlaceQuery(state, feed.school)));
  return (
    <SearchView
      mode={mode}
      state={state}
      school={feed.school}
      initial={initial}
      fetchedAt={fetchedAt}
      suggestions={feed.popular.slice(0, 2)}
    />
  );
}

/** The first page, or null when the API failed; stamped so the view can say how old it is. */
async function loadFirstPage(load: () => Promise<Page<Place>>): Promise<{ initial: Page<Place> | null; fetchedAt: number }> {
  await connection(); // the timestamp below is per request
  try {
    const initial = await load();
    return { initial, fetchedAt: Date.now() };
  } catch {
    return { initial: null, fetchedAt: Date.now() };
  }
}

function SearchFallback() {
  return (
    <Stack gap={5} style={{ paddingTop: "var(--space-6)" }}>
      <LoadingNote>Loading places…</LoadingNote>
      <Placeholder height={56} radius="control" />
      <Placeholder height={44} radius="control" />
      <Placeholder />
      <Placeholder />
    </Stack>
  );
}

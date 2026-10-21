import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { cache, Suspense } from "react";
import { HoursList, OpenNow } from "@/components/HoursList/HoursList";
import { Aside, Columns, Container, Crumbs, LoadingNote, MainColumn, Placeholder, Stack } from "@/components/Layout/Layout";
import { Map as PlaceMap } from "@/components/Map/Map";
import { RatingSummary } from "@/components/RatingSummary/RatingSummary";
import { Tile } from "@/components/Tile/Tile";
import { isApiError } from "@/lib/api/errors";
import { getMe, getServerApi } from "@/lib/api/server";
import { viewerOf } from "@/lib/auth";
import { requestTime } from "@/lib/requestTime";
import { CATEGORY_LABEL, CATEGORY_SINGULAR, distance, price } from "@/lib/format";
import { filterHref } from "@/lib/filters";
import { PlaceActions } from "./PlaceActions";
import { ReviewList } from "./ReviewList";
import styles from "./place.module.css";

type Params = Promise<{ id: string }>;

/** Deduped per request: generateMetadata and the page both need the place. */
const loadPlace = cache(async (id: string) => {
  const api = await getServerApi();
  try {
    return await api.place(id);
  } catch (e) {
    if (isApiError(e) && (e.status === 404 || e.status === 422)) notFound();
    throw e;
  }
});

export async function generateMetadata({ params }: { params: Params }): Promise<Metadata> {
  const { id } = await params;
  try {
    const place = await loadPlace(id);
    return { title: place.name, description: place.top_quote?.text ?? `${place.name} on Corner` };
  } catch {
    return { title: "Place" };
  }
}

export default function PlacePage({ params }: { params: Params }) {
  return (
    <Container>
      <Suspense fallback={<PlaceFallback />}>
        <PlaceContent params={params} />
      </Suspense>
    </Container>
  );
}

async function PlaceContent({ params }: { params: Params }) {
  const { id } = await params;
  const api = await getServerApi();
  const [place, me] = await Promise.all([loadPlace(id), getMe(api).catch(() => null)]);
  const [reviews, ownReview, renderedAt] = await Promise.all([
    api.reviews(id, { sort: "helpful" }).catch(() => ({ items: [], next_cursor: null })),
    place.viewer_review_id ? api.review(place.viewer_review_id).catch(() => null) : Promise.resolve(null),
    requestTime(),
  ]);

  const street = place.address.split(",")[0];
  const away = place.kind === "facility" ? "On campus" : distance(place.distance_mi) ? `${distance(place.distance_mi)} from campus` : null;
  const meta = [place.subcategory || CATEGORY_SINGULAR[place.category], price(place.price_level), street, away].filter(Boolean).join(" · ");
  const schoolName = place.school?.name ?? me?.school?.name;
  const viewer = viewerOf(me);

  return (
    <>
      <Crumbs
        items={[
          { label: schoolName ? `Near ${schoolName}` : "Places", href: "/" },
          { label: CATEGORY_LABEL[place.category], href: filterHref(place.category) },
        ]}
      />

      <section className={styles.hero} aria-labelledby="place-name">
        <div className={styles.heroMain}>
          {place.status === "pending" ? <p className={styles.pending}>Pending review. Only you and admins can see this place for now.</p> : null}
          <h1 id="place-name" className={styles.name}>
            {place.name}
          </h1>
          <p className={styles.meta}>
            {meta}
            <OpenNow hours={place.hours} className={styles.open} />
          </p>
          <div className={styles.actions}>
            <PlaceActions place={place} signedIn={!!me} ownReviewEditableUntil={ownReview?.editable_until ?? null}
              renderedAt={renderedAt}
            />
          </div>
        </div>
        <div className={styles.heroSide}>
          <RatingSummary average={place.rating_avg} count={place.review_count} distribution={place.rating_distribution} />
        </div>
      </section>

      <Columns gap={7} className={styles.body}>
        <MainColumn>
          <ReviewList placeId={place.id} placeName={place.name} initial={reviews}
            viewer={viewer}
            accessUntil={me?.access_until}
            renderedAt={renderedAt}
            open={place.status === "active"}
          />
        </MainColumn>
        <Aside label="Details">
          {place.student_discount ? (
            <Tile tone="accent" title="Student deal">
              <p>{place.student_discount}. Show your Corner profile before you pay.</p>
            </Tile>
          ) : null}
          {place.hours?.length ? (
            <Tile title="Hours">
              <HoursList hours={place.hours} />
            </Tile>
          ) : null}
          <PlaceMap
            label={`Map showing ${place.name}`}
            points={[{ id: place.id, lat: place.lat, lng: place.lng, label: place.name }]}
            caption={place.address}
          />
        </Aside>
      </Columns>
    </>
  );
}

function PlaceFallback() {
  return (
    <Stack gap={5} style={{ paddingTop: "var(--space-6)" }}>
      <LoadingNote>Loading place…</LoadingNote>
      <Placeholder height={64} radius="control" />
      <Placeholder height={160} radius="hero" />
      <Placeholder />
    </Stack>
  );
}

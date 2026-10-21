import { Suspense } from "react";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { FeaturedPlace } from "@/components/FeaturedPlace/FeaturedPlace";
import { FilterChips } from "@/components/FilterChips/FilterChips";
import { Container, LoadingNote, PageHead, Placeholder, Section, TextLink, TileGrid } from "@/components/Layout/Layout";
import { PlaceTile, PromoTile } from "@/components/PlaceTile/PlaceTile";
import { getMe, getServerApi } from "@/lib/api/server";
import { FILTERS } from "@/lib/format";
import { filterHref } from "@/lib/filters";
import styles from "./page.module.css";

export default function HomePage() {
  return (
    <Container>
      <Suspense fallback={<FeedFallback />}>
        <Feed />
      </Suspense>
    </Container>
  );
}

async function Feed() {
  const api = await getServerApi();
  const [feed, me] = await Promise.all([api.feed(), getMe(api).catch(() => null)]);
  const school = feed.school;
  const chips = FILTERS.map((f) => ({ ...f, href: filterHref(f.value) }));

  return (
    <>
      <section className={styles.intro}>
        <PageHead title={`Near ${school.name}`}>Places within a mile, reviewed by verified students.</PageHead>
        <FilterChips label="Categories" options={chips} value="all" className={styles.chips} />
      </section>

      {feed.featured ? (
        <div className={styles.featured}>
          <FeaturedPlace place={feed.featured.place} quote={feed.featured.quote} />
        </div>
      ) : null}

      {feed.popular.length ? (
        <Section title="Most reviewed this week" id="popular" className={styles.section}>
          <TileGrid>
            {feed.popular.map((p) => (
              <PlaceTile key={p.id} place={p} />
            ))}
            {!me ? (
              <PromoTile
                href="/signup"
                title="Been somewhere?"
                text="Verify your school email to post. College and high-school addresses both work."
                cta="Get verified"
              />
            ) : null}
          </TileGrid>
        </Section>
      ) : (
        <EmptyState
          title={`No places near ${school.name} yet`}
          action={<Button href="/places/new">Add a place</Button>}
        >
          Know one? Add it and be the first to review it.
        </EmptyState>
      )}

      {feed.deals.length ? (
        <Section
          title="Student deals"
          id="deals"
          className={styles.section}
          action={<TextLink href="/deals">See all deals</TextLink>}
        >
          <TileGrid>
            {feed.deals.map((p) => (
              <PlaceTile key={p.id} place={p} />
            ))}
          </TileGrid>
        </Section>
      ) : null}
    </>
  );
}

function FeedFallback() {
  return (
    <>
      <LoadingNote>Loading places…</LoadingNote>
      <section className={styles.intro}>
        <Placeholder height={56} radius="control" />
      </section>
      <div className={styles.featured}>
        <Placeholder height={240} radius="hero" />
      </div>
      <div className={styles.section}>
        <TileGrid>
          <Placeholder />
          <Placeholder />
          <Placeholder />
        </TileGrid>
      </div>
    </>
  );
}

import Link from "next/link";
import { RatingPill } from "@/components/RatingPill/RatingPill";
import type { Place } from "@/lib/api/types";
import { count, placeMeta } from "@/lib/format";
import styles from "./PlaceTile.module.css";

interface PlaceTileProps {
  place: Place;
  /** Where the tile links. Defaults to the place page. */
  href?: string;
  headingLevel?: 2 | 3;
}

/**
 * Surface tile: meta line and rating pill, Headline name, a quote, and a footer with
 * the reviewer and the deal. The whole tile is the link target.
 */
export function PlaceTile({ place, href, headingLevel = 3 }: PlaceTileProps) {
  const Heading = headingLevel === 2 ? "h2" : "h3";
  const quote = place.top_quote;
  return (
    <article className={styles.tile}>
      <div className={styles.top}>
        <span className={styles.meta}>{placeMeta(place)}</span>
        <RatingPill value={place.rating_avg} on="bg" />
      </div>
      <Heading className={styles.name}>
        <Link href={href ?? `/places/${place.id}`} className={styles.link}>
          {place.name}
        </Link>
      </Heading>
      {quote ? <p className={styles.quote}>“{quote.text}”</p> : null}
      <div className={styles.footer}>
        <span className={styles.by}>
          {quote ? `${quote.author.display_name} · ` : ""}
          {place.review_count ? `${count(place.review_count)} ${place.review_count === 1 ? "review" : "reviews"}` : "No reviews yet"}
        </span>
        {place.student_discount ? <span className={styles.deal}>{place.student_discount}</span> : null}
      </div>
    </article>
  );
}

interface PromoTileProps {
  title: string;
  text: string;
  cta: string;
  href: string;
}

/** Ink tile that sits in a PlaceTile grid, like "Been somewhere? Get verified". */
export function PromoTile({ title, text, cta, href }: PromoTileProps) {
  return (
    <Link href={href} className={styles.promo}>
      <span className={styles.promoTitle}>{title}</span>
      <span className={styles.promoText}>{text}</span>
      <span className={styles.promoCta}>
        {cta} <span aria-hidden="true">→</span>
      </span>
    </Link>
  );
}

import Link from "next/link";
import { Badge } from "@/components/Badge/Badge";
import { RatingPill } from "@/components/RatingPill/RatingPill";
import type { Place, Quote } from "@/lib/api/types";
import { authorLine, placeMeta, plural } from "@/lib/format";
import styles from "./FeaturedPlace.module.css";

interface FeaturedPlaceProps {
  place: Place;
  quote: Quote;
}

/** The accent-soft hero holding a pull-quote from a top review. One per page. */
export function FeaturedPlace({ place, quote }: FeaturedPlaceProps) {
  return (
    <article className={styles.hero} aria-labelledby="featured-name">
      <figure className={styles.figure}>
        <blockquote className={styles.quote}>
          <p>“{quote.text}”</p>
        </blockquote>
        <figcaption className={styles.author}>{authorLine(quote.author)}</figcaption>
      </figure>
      <div className={styles.side}>
        <h2 id="featured-name" className={styles.name}>
          <Link href={`/places/${place.id}`}>{place.name}</Link>
        </h2>
        <div className={styles.meta}>{placeMeta(place)}</div>
        <div className={styles.pills}>
          <RatingPill value={place.rating_avg} on="bg" size="large" suffix={plural(place.review_count, "review")} />
          {place.student_discount ? (
            <Badge tone="deal" size="large">
              {place.student_discount}
            </Badge>
          ) : null}
        </div>
      </div>
    </article>
  );
}

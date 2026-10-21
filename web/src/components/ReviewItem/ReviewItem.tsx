import { Avatar } from "@/components/Avatar/Avatar";
import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";
import { Stars } from "@/components/StarRating/StarRating";
import type { Review } from "@/lib/api/types";
import { monthLabel, shortDate } from "@/lib/format";
import styles from "./ReviewItem.module.css";

interface ReviewItemProps {
  review: Review;
  /** Used to drop the year from dates in the current year. */
  currentYear?: number;
  onHelpful?: (review: Review) => void;
  onReport?: (review: Review) => void;
  onDelete?: (review: Review) => void;
  /** False hides the Helpful toggle (e.g. signed out); the count still shows. */
  canVote?: boolean;
  busy?: boolean;
  /** Current time in ms, ticking on the client. Null on the server, which hides the edit countdown. */
  now?: number | null;
  /** Where the author edits this review; shown only while editable_until is ahead of `now`. */
  editHref?: string;
}

/** Avatar, name, verified badge, date, stars, body, then Helpful and Report. */
export function ReviewItem({ review, currentYear, onHelpful, onReport, onDelete, canVote = true, busy, now, editHref }: ReviewItemProps) {
  const { author } = review;
  const msLeft = review.is_mine && review.editable_until && now ? new Date(review.editable_until).getTime() - now : 0;
  const minutesLeft = Math.ceil(msLeft / 60_000);
  const headingId = `review-${review.id}-by`;
  return (
    <article id={`review-${review.id}`} className={styles.item} aria-labelledby={headingId}>
      <Avatar name={author.display_name} size={44} />
      <div className={styles.main}>
        <div className={styles.head}>
          <h3 id={headingId} className={styles.name}>
            {author.display_name}
          </h3>
          <Badge tone="verified">Verified · {author.school_name}</Badge>
          {review.is_mine ? <Badge tone="neutral">Your review</Badge> : null}
          <span className={styles.date}>
            <time dateTime={review.created_at}>{shortDate(review.created_at, currentYear)}</time>
            {review.edited_at ? " · Edited" : null}
          </span>
        </div>
        <Stars value={review.rating} className={styles.stars} />
        <p className={styles.body}>{review.body}</p>
        {review.visited_month || review.used_deal ? (
          <p className={styles.extra}>
            {[review.visited_month && `Visited ${monthLabel(review.visited_month)}`, review.used_deal && "Used the student deal"]
              .filter(Boolean)
              .join(" · ")}
          </p>
        ) : null}
        <div className={styles.actions}>
          {review.is_mine ? (
            <>
              <span className={styles.count}>Helpful · {review.helpful_count}</span>
              {editHref && msLeft > 0 ? (
                <Button size={36} variant="ghost" href={editHref}>
                  Edit · {minutesLeft} min left
                </Button>
              ) : null}
              {onDelete ? (
                <Button size={36} variant="ghost" onClick={() => onDelete(review)} disabled={busy}>
                  Delete review
                </Button>
              ) : null}
            </>
          ) : (
            <>
              {canVote && onHelpful ? (
                <Button
                  size={36}
                  variant="secondary"
                  aria-pressed={review.viewer_voted}
                  onClick={() => onHelpful(review)}
                  disabled={busy}
                >
                  Helpful · {review.helpful_count}
                </Button>
              ) : (
                <span className={styles.count}>Helpful · {review.helpful_count}</span>
              )}
              {onReport ? (
                <button type="button" className={styles.report} onClick={() => onReport(review)}>
                  Report
                </button>
              ) : null}
            </>
          )}
        </div>
      </div>
    </article>
  );
}

import { Stars } from "@/components/StarRating/StarRating";
import type { RatingDistribution } from "@/lib/api/types";
import { plural, rating } from "@/lib/format";
import styles from "./RatingSummary.module.css";

interface RatingSummaryProps {
  average: number | null;
  count: number;
  distribution: RatingDistribution;
}

/** Big average, stars and count, plus a 5-to-1 distribution. */
export function RatingSummary({ average, count, distribution }: RatingSummaryProps) {
  const rows = (["5", "4", "3", "2", "1"] as const).map((k) => ({ stars: k, n: distribution[k] ?? 0 }));
  const max = Math.max(1, ...rows.map((r) => r.n));
  return (
    <section className={styles.summary} aria-label="Rating summary">
      <div className={styles.score}>
        <div className={styles.number}>{rating(average) ?? "–"}</div>
        {average !== null ? <Stars value={average} className={styles.stars} /> : null}
        <div className={styles.count}>{count ? plural(count, "review") : "No reviews yet"}</div>
      </div>
      <dl className={styles.bars}>
        {rows.map((r) => (
          <div key={r.stars} className={styles.row}>
            <dt className={styles.label}>
              <span aria-hidden="true">{r.stars}</span>
              <span className="visually-hidden">{r.stars === "1" ? "1 star" : `${r.stars} stars`}</span>
            </dt>
            <dd className={styles.track}>
              <span className={styles.fill} style={{ width: `${Math.round((r.n / max) * 100)}%` }} />
              <span className="visually-hidden">{plural(r.n, "review")}</span>
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

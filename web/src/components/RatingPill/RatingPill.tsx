import { rating as fmt } from "@/lib/format";
import styles from "./RatingPill.module.css";

interface RatingPillProps {
  value: number | null;
  /** Fill: "bg" when the pill sits on a surface tile, "surface" when it sits on bg. */
  on?: "bg" | "surface";
  /** Extra text after the number, like "212 reviews". */
  suffix?: string;
  size?: "default" | "large";
}

/** "★ 4.6", or "New" when there are no reviews yet. */
export function RatingPill({ value, on = "bg", suffix, size = "default" }: RatingPillProps) {
  const n = fmt(value);
  const label = n ? `Rated ${n} out of 5${suffix ? `, ${suffix}` : ""}` : `No ratings yet${suffix ? `, ${suffix}` : ""}`;
  return (
    <span className={[styles.pill, styles[on], size === "large" && styles.large].filter(Boolean).join(" ")} aria-label={label} role="img">
      {n ? (
        <>
          <span className={styles.star} aria-hidden="true">
            ★
          </span>{" "}
          {n}
        </>
      ) : (
        "New"
      )}
      {suffix ? ` · ${suffix}` : null}
    </span>
  );
}

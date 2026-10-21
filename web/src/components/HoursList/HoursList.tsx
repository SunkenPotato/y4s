"use client";

import { useSyncExternalStore } from "react";
import type { HoursRow } from "@/lib/api/types";
import { openStatus, todayRowIndex } from "./hours";
import styles from "./HoursList.module.css";

const subscribe = () => () => {};
/** The current minute on the client, null during SSR so markup matches on hydrate. */
function useNow(): Date | null {
  const minute = useSyncExternalStore(
    subscribe,
    () => Math.floor(Date.now() / 60000),
    () => null,
  );
  return minute === null ? null : new Date(minute * 60000);
}

/** Opening hours with today's row in bold. */
export function HoursList({ hours }: { hours: HoursRow[] }) {
  const now = useNow();
  const today = now ? todayRowIndex(hours, now) : -1;
  return (
    <dl className={styles.list}>
      {hours.map((h, i) => (
        <div key={`${h.days}-${i}`} className={[styles.row, i === today && styles.today].filter(Boolean).join(" ")}>
          <dt>
            {h.days}
            {i === today ? <span className="visually-hidden"> (today)</span> : null}
          </dt>
          <dd>{h.time}</dd>
        </div>
      ))}
    </dl>
  );
}

/** "Open until 10 pm" in accent-ink, computed on the client from the viewer's clock. */
export function OpenNow({ hours, className }: { hours: HoursRow[] | null; className?: string }) {
  const now = useNow();
  const status = now ? openStatus(hours, now) : null;
  if (!status) return null;
  return <span className={[status.open ? styles.open : styles.closed, className].filter(Boolean).join(" ")}>{status.text}</span>;
}

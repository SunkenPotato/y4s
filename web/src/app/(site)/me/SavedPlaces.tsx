"use client";

import { useState } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { PlaceTile } from "@/components/PlaceTile/PlaceTile";
import { api, isApiError } from "@/lib/api/client";
import type { Page, Place } from "@/lib/api/types";
import styles from "./profile.module.css";

/** The signed-in user's saved places, most recently saved first. */
export function SavedPlaces({ initial }: { initial: Page<Place> | null }) {
  const [items, setItems] = useState(initial?.items ?? []);
  const [next, setNext] = useState(initial?.next_cursor ?? null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(initial ? null : "Couldn't load your saved places.");

  async function more(cursor: string | null) {
    setLoading(true);
    setError(null);
    try {
      const page = await api.saved(cursor);
      setItems((prev) => (cursor ? [...prev, ...page.items] : page.items));
      setNext(page.next_cursor);
    } catch (e) {
      setError(isApiError(e) ? e.message : "Couldn't load your saved places.");
    } finally {
      setLoading(false);
    }
  }

  if (error && items.length === 0) {
    return (
      <Alert
        tone="error"
        action={
          <Button size={36} variant="secondary" onClick={() => more(null)} loading={loading} loadingLabel="Retrying…">
            Retry
          </Button>
        }
      >
        {error}
      </Alert>
    );
  }

  if (items.length === 0) {
    return (
      <EmptyState icon="bookmark" title="Nothing saved yet" action={<Button href="/">Find places</Button>}>
        Tap Save on any place to keep it here.
      </EmptyState>
    );
  }

  return (
    <>
      <div className={styles.saved}>
        {items.map((p) => (
          <PlaceTile key={p.id} place={p} />
        ))}
      </div>
      {error ? <Alert tone="error">{error}</Alert> : null}
      {next ? (
        <div className={styles.more}>
          <Button variant="secondary" onClick={() => more(next)} loading={loading} loadingLabel="Loading…">
            Show more saved places
          </Button>
        </div>
      ) : null}
    </>
  );
}

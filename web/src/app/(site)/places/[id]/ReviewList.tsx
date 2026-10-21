"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Section } from "@/components/Layout/Layout";
import { ReviewItem } from "@/components/ReviewItem/ReviewItem";
import { Segmented } from "@/components/Segmented/Segmented";
import { useToast } from "@/components/Toast/Toast";
import { api, isApiError } from "@/lib/api/client";
import type { Page, Review, ReviewSort } from "@/lib/api/types";
import { loginHref } from "@/lib/auth";
import { useCurrentYear } from "@/lib/hooks/useCurrentYear";
import { ReportDialog } from "./ReportDialog";
import styles from "./place.module.css";

const SORTS = [
  { value: "helpful", label: "Most helpful" },
  { value: "new", label: "Newest" },
  { value: "lowest", label: "Lowest" },
] as const;

interface ReviewListProps {
  placeId: string;
  placeName: string;
  initial: Page<Review>;
  viewer: "signed_out" | "unverified" | "verified";
  /** False for places still pending approval. */
  open?: boolean;
}

export function ReviewList({ placeId, placeName, initial, viewer, open = true }: ReviewListProps) {
  const router = useRouter();
  const toast = useToast();
  const year = useCurrentYear();
  const [sort, setSort] = useState<ReviewSort>("helpful");
  const [items, setItems] = useState(initial.items);
  const [next, setNext] = useState(initial.next_cursor);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [voting, setVoting] = useState<Set<string>>(new Set());
  const [reporting, setReporting] = useState<Review | null>(null);

  async function load(nextSort: ReviewSort, cursor: string | null) {
    setLoading(true);
    setError(null);
    try {
      const page = await api.reviews(placeId, { sort: nextSort, cursor });
      setItems((prev) => (cursor ? [...prev, ...page.items] : page.items));
      setNext(page.next_cursor);
    } catch (e) {
      setError(isApiError(e) ? e.message : "Couldn't load reviews. Try again.");
    } finally {
      setLoading(false);
    }
  }

  function changeSort(value: ReviewSort) {
    if (value === sort) return;
    setSort(value);
    void load(value, null);
  }

  function patch(id: string, update: Partial<Review>) {
    setItems((prev) => prev.map((r) => (r.id === id ? { ...r, ...update } : r)));
  }

  async function toggleHelpful(review: Review) {
    if (viewer === "signed_out") {
      router.push(loginHref(`/places/${placeId}`));
      return;
    }
    if (viewer === "unverified") {
      toast.show({ message: "Verify your school email to vote." });
      return;
    }
    if (voting.has(review.id)) return;
    const want = !review.viewer_voted;
    // Optimistic: flip now, reconcile with the server's count, roll back on failure.
    patch(review.id, { viewer_voted: want, helpful_count: review.helpful_count + (want ? 1 : -1) });
    setVoting((s) => new Set(s).add(review.id));
    try {
      const res = want ? await api.vote(review.id) : await api.unvote(review.id);
      patch(review.id, res);
    } catch (e) {
      patch(review.id, { viewer_voted: review.viewer_voted, helpful_count: review.helpful_count });
      toast.show({ message: isApiError(e) ? e.message : "Couldn't save your vote. Try again." });
    } finally {
      setVoting((s) => {
        const n = new Set(s);
        n.delete(review.id);
        return n;
      });
    }
  }

  async function remove(review: Review) {
    const index = items.findIndex((r) => r.id === review.id);
    setItems((prev) => prev.filter((r) => r.id !== review.id));
    try {
      await api.deleteReview(review.id);
    } catch (e) {
      setItems((prev) => [...prev.slice(0, index), review, ...prev.slice(index)]);
      toast.show({ message: isApiError(e) ? e.message : "Couldn't delete your review. Try again." });
      return;
    }
    router.refresh();
    toast.show({
      message: "Review deleted",
      onUndo: async () => {
        try {
          const restored = await api.restoreReview(review.id);
          setItems((prev) => [...prev.slice(0, index), restored, ...prev.slice(index)]);
          router.refresh();
        } catch (e) {
          toast.show({ message: isApiError(e) ? e.message : "Couldn't restore your review." });
        }
      },
    });
  }

  function report(review: Review) {
    if (viewer === "signed_out") {
      router.push(loginHref(`/places/${placeId}`));
      return;
    }
    setReporting(review);
  }

  return (
    <Section
      title="What students say"
      id="reviews"
      action={
        items.length || sort !== "helpful" ? (
          <Segmented label="Sort reviews" options={SORTS} value={sort} onChange={changeSort} />
        ) : null
      }
    >
      {error ? (
        <Alert
          tone="error"
          action={
            <Button size={36} variant="secondary" onClick={() => load(sort, null)}>
              Retry
            </Button>
          }
        >
          {error}
        </Alert>
      ) : null}

      {items.length === 0 && !loading ? (
        <EmptyState
          icon="pen"
          title="No reviews yet"
          action={open ? <Button href={`/places/${placeId}/review`}>Write a review</Button> : undefined}
        >
          {open
            ? `Been to ${placeName}? Be the first to tell other students about it.`
            : "Reviews open once an admin approves this place."}
        </EmptyState>
      ) : (
        <div className={styles.reviews} aria-busy={loading || undefined}>
          {items.map((r) => (
            <ReviewItem
              key={r.id}
              review={r}
              currentYear={year}
              canVote={viewer !== "signed_out"}
              onHelpful={toggleHelpful}
              onReport={report}
              onDelete={remove}
              busy={voting.has(r.id)}
            />
          ))}
        </div>
      )}

      {next ? (
        <div className={styles.more}>
          <Button variant="secondary" onClick={() => load(sort, next)} loading={loading} loadingLabel="Loading…">
            Show more reviews
          </Button>
        </div>
      ) : null}

      <ReportDialog
        open={reporting !== null}
        onClose={() => setReporting(null)}
        target={reporting ? { type: "review", id: reporting.id, name: reporting.author.display_name } : null}
      />
    </Section>
  );
}

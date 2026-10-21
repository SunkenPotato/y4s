"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { Checkbox } from "@/components/Checkbox/Checkbox";
import { Field } from "@/components/Field/Field";
import { Aside, Columns, MainColumn, Muted, PageHead, Row, Stack, TextLink } from "@/components/Layout/Layout";
import { Select } from "@/components/Select/Select";
import { StarInput } from "@/components/StarRating/StarRating";
import { Textarea, formatCounter } from "@/components/Textarea/Textarea";
import { Tile } from "@/components/Tile/Tile";
import { useToast } from "@/components/Toast/Toast";
import { ExpiredAlert } from "@/flows/access/ExpiredAlert";
import { api, isApiError } from "@/lib/api/client";
import type { Rating, Review } from "@/lib/api/types";
import { monthLabel, recentMonths } from "@/lib/format";
import { useNow } from "@/lib/hooks/useNow";
import styles from "./review.module.css";

const MIN = 50;
const MAX = 2000;

interface Draft {
  rating: number | null;
  body: string;
  visited: string;
  usedDeal: boolean;
}

interface ReviewFormProps {
  place: { id: string; name: string; hasDeal: boolean };
  author: { displayName: string; schoolName: string };
  /** Edit mode: pre-fills the form and saves with PATCH. Drafts aren't stored while editing. */
  existing?: Review;
  /** Server render time, so a closed edit window shows before hydration. */
  renderedAt?: number;
}

type FormError = { title: string; text?: string; backLink?: boolean; expiredUntil?: string };

const draftKey = (placeId: string) => `corner:review-draft:${placeId}`;

function readDraft(placeId: string): Partial<Draft> | null {
  try {
    const raw = window.localStorage.getItem(draftKey(placeId));
    return raw ? (JSON.parse(raw) as Partial<Draft>) : null;
  } catch {
    return null;
  }
}

function bodyError(body: string): string | null {
  const n = body.trim().length;
  if (n >= MIN) return null;
  return `Write at least 50 characters so it's useful. ${MIN - n} more to go.`;
}

export function ReviewForm({ place, author, existing, renderedAt }: ReviewFormProps) {
  const router = useRouter();
  const toast = useToast();
  const editing = !!existing;
  const now = useNow(1000, renderedAt ?? null);
  const [months] = useState(() => {
    const list = recentMonths(new Date(), 12);
    // Keep an older visited month selectable when editing.
    if (existing?.visited_month && !list.includes(existing.visited_month)) list.push(existing.visited_month);
    return list;
  });
  const [rating, setRating] = useState<number | null>(existing?.rating ?? null);
  const [body, setBody] = useState(existing?.body ?? "");
  const [visited, setVisited] = useState<string>(existing ? (existing.visited_month ?? "") : months[0]);
  const [usedDeal, setUsedDeal] = useState(existing?.used_deal ?? false);
  const [errors, setErrors] = useState<{ rating?: string; body?: string; visited_month?: string }>({});
  const [formError, setFormError] = useState<FormError | null>(null);
  const [state, setState] = useState<"idle" | "posting" | "posted">("idle");
  const [showErrors, setShowErrors] = useState(false);
  const loaded = useRef(false);
  const bodyRef = useRef<HTMLTextAreaElement>(null);
  const starsRef = useRef<HTMLDivElement>(null);

  // Restore a saved draft once, after hydration (localStorage isn't available on the server).
  useEffect(() => {
    if (editing) return;
    const d = readDraft(place.id);
    if (d) {
      /* eslint-disable react-hooks/set-state-in-effect -- one-time restore from localStorage */
      if (typeof d.rating === "number") setRating(d.rating);
      if (typeof d.body === "string") setBody(d.body);
      if (typeof d.visited === "string") setVisited(d.visited);
      if (typeof d.usedDeal === "boolean") setUsedDeal(d.usedDeal);
      /* eslint-enable react-hooks/set-state-in-effect */
    }
    loaded.current = true;
  }, [place.id, editing]);

  // Save the draft as the student types.
  useEffect(() => {
    if (editing || !loaded.current || state === "posted") return;
    const t = window.setTimeout(() => {
      try {
        const draft: Draft = { rating, body, visited, usedDeal };
        if (!rating && !body.trim()) window.localStorage.removeItem(draftKey(place.id));
        else window.localStorage.setItem(draftKey(place.id), JSON.stringify(draft));
      } catch {
        /* storage full or blocked; the draft just isn't saved */
      }
    }, 400);
    return () => window.clearTimeout(t);
  }, [rating, body, visited, usedDeal, place.id, state, editing]);

  // The 15-minute edit window, checked against the viewer's clock.
  const windowClosed = editing && !!now && state !== "posted" && (!existing.editable_until || new Date(existing.editable_until).getTime() <= now);

  const ratingMsg = errors.rating ?? (showErrors && !rating ? "Pick a rating from 1 to 5 stars." : null);
  const bodyMsg = showErrors && body.trim().length < MIN ? bodyError(body) : (errors.body ?? null);

  function changeRating(n: number) {
    setRating(n);
    setErrors(({ rating: _r, ...rest }) => rest);
  }

  function changeBody(value: string) {
    setBody(value);
    if (errors.body) setErrors(({ body: _b, ...rest }) => rest);
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (state !== "idle") return;
    setShowErrors(true);
    setFormError(null);
    setErrors({});
    const localBody = bodyError(body);
    if (!rating || localBody) {
      if (!rating) starsRef.current?.querySelector<HTMLButtonElement>('[tabindex="0"]')?.focus();
      else bodyRef.current?.focus();
      return;
    }

    setState("posting");
    if (existing) return saveEdit(existing);
    try {
      const review = await api.createReview(place.id, {
        rating: rating as Rating,
        body: body.trim(),
        visited_month: visited || null,
        used_deal: place.hasDeal ? usedDeal : false,
      });
      setState("posted");
      try {
        window.localStorage.removeItem(draftKey(place.id));
      } catch {
        /* ignore */
      }
      toast.show({
        message: "Review posted",
        onUndo: async () => {
          try {
            await api.deleteReview(review.id);
            router.refresh();
          } catch (err) {
            toast.show({ message: isApiError(err) ? err.message : "Couldn't remove your review." });
          }
        },
      });
      router.push(`/places/${place.id}#review-${review.id}`);
      router.refresh();
    } catch (err) {
      setState("idle");
      if (!isApiError(err)) {
        setFormError({ title: "Couldn't post your review", text: "Check your connection and try again." });
        return;
      }
      if (err.code === "validation") {
        setErrors(err.fields);
        if (err.fields.rating) starsRef.current?.querySelector<HTMLButtonElement>('[tabindex="0"]')?.focus();
        else if (err.fields.body) bodyRef.current?.focus();
        if (!err.fields.rating && !err.fields.body) setFormError({ title: "Check your review", text: err.message });
      } else if (err.code === "already_reviewed") {
        setFormError({ title: "You've already reviewed this place", text: err.message });
      } else if (err.code === "access_expired") {
        await showExpired(err.message);
      } else if (err.code === "not_verified") {
        setFormError({ title: "Verify your school email to post", text: err.message });
      } else if (err.status === 401) {
        setFormError({ title: "You're logged out", text: "Log in again to post. Your draft is saved." });
      } else {
        setFormError({ title: "Couldn't post your review", text: err.message });
      }
    }
  }

  // access_expired: show the standard message, which needs the viewer's access_until.
  async function showExpired(fallback: string) {
    const me = await api.me().catch(() => null);
    setFormError({ title: fallback, expiredUntil: me?.access_until });
  }

  async function saveEdit(review: Review) {
    try {
      const updated = await api.updateReview(review.id, {
        rating: rating as Rating,
        body: body.trim(),
        visited_month: visited || null,
        used_deal: place.hasDeal ? usedDeal : false,
      });
      setState("posted");
      toast.show({ message: "Review updated" });
      router.push(`/places/${place.id}#review-${updated.id}`);
      router.refresh();
    } catch (err) {
      setState("idle");
      if (!isApiError(err)) {
        setFormError({ title: "Couldn't save your changes", text: "Check your connection and try again." });
      } else if (err.code === "validation") {
        setErrors(err.fields);
        if (!err.fields.rating && !err.fields.body) setFormError({ title: "Check your review", text: err.message });
      } else if (err.code === "edit_window_closed") {
        setFormError({ title: "Editing time is over", text: err.message, backLink: true });
      } else if (err.code === "access_expired") {
        await showExpired(err.message);
      } else {
        setFormError({ title: "Couldn't save your changes", text: err.message });
      }
    }
  }

  const closedError: FormError | null = windowClosed
    ? { title: "Editing time is over", text: "Reviews can be edited for 15 minutes after posting.", backLink: true }
    : null;
  const shownError = formError ?? closedError;
  const idleLabel = editing ? "Save changes" : "Post review";
  const busyLabel = editing ? "Saving…" : "Posting…";
  const doneLabel = editing ? "Changes saved" : "Review posted";

  return (
    <Columns gap={7} className={styles.layout}>
      <MainColumn>
        <form onSubmit={submit} noValidate className={styles.form}>
          <PageHead title={editing ? `Edit your review of ${place.name}` : `Review ${place.name}`} size="title">
            Posting as {author.displayName} · Verified at {author.schoolName}
          </PageHead>

          {shownError?.expiredUntil ? (
            <ExpiredAlert accessUntil={shownError.expiredUntil} />
          ) : shownError ? (
            <Alert
              tone="error"
              title={shownError.title}
              action={shownError.backLink ? <TextLink href={`/places/${place.id}`}>Back to {place.name}</TextLink> : undefined}
            >
              {shownError.text}
            </Alert>
          ) : null}

          <fieldset className={styles.fieldset}>
            <legend className={styles.legend}>Your rating</legend>
            <div ref={starsRef} className={styles.stars}>
              <StarInput
                value={rating}
                onChange={changeRating}
                size={48}
                invalid={!!ratingMsg}
                errorId={ratingMsg ? "rating-error" : undefined}
              />
            </div>
            {ratingMsg ? (
              <p id="rating-error" className={styles.error}>
                {ratingMsg}
              </p>
            ) : null}
          </fieldset>

          <Field
            label="What should other students know?"
            labelSize="large"
            description="Prices, wait times, noise, whether the student deal actually works."
            error={bodyMsg}
            counter={formatCounter(body.length, MAX)}
          >
            <Textarea ref={bodyRef} value={body} maxLength={MAX} rows={6} onChange={(e) => changeBody(e.target.value)} />
          </Field>

          <div className={styles.pair}>
            <Field label="When did you go?" error={errors.visited_month} className={styles.half}>
              <Select value={visited} onChange={(e) => setVisited(e.target.value)}>
                {months.map((m) => (
                  <option key={m} value={m}>
                    {monthLabel(m)}
                  </option>
                ))}
                <option value="">Earlier</option>
              </Select>
            </Field>
            {place.hasDeal ? (
              <div className={`${styles.half} ${styles.checkbox}`}>
                <Checkbox label="I used the student deal" checked={usedDeal} onChange={(e) => setUsedDeal(e.target.checked)} />
              </div>
            ) : null}
          </div>

          <Row gap={3}>
            <Button
              type="submit"
              loading={state === "posting"}
              loadingLabel={busyLabel}
              disabled={state === "posted" || windowClosed}
            >
              {state === "posted" ? doneLabel : idleLabel}
            </Button>
            <TextLink href={`/places/${place.id}`}>Cancel</TextLink>
            {!editing ? (
              <Muted small as="span">
                Your draft is saved automatically.
              </Muted>
            ) : null}
          </Row>
        </form>
      </MainColumn>

      <Aside label="Tips">
        <Stack gap={4}>
          <Tile title="Good reviews">
            <ul className={styles.tips}>
              <li>Say what you ordered or used.</li>
              <li>Mention prices and when you went.</li>
              <li>Keep it about the place, not people who work there.</li>
            </ul>
          </Tile>
          <Tile tone="accent" title="Shown with your review">
            <p>
              {author.displayName} · Verified · {author.schoolName}. Your email is never shown.
            </p>
          </Tile>
        </Stack>
      </Aside>
    </Columns>
  );
}

import type { Metadata } from "next";
import { notFound, redirect } from "next/navigation";
import { Suspense } from "react";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Container, Crumbs, LoadingNote, PageHead, Placeholder, Stack } from "@/components/Layout/Layout";
import { ExpiredAlert } from "@/flows/access/ExpiredAlert";
import { isApiError } from "@/lib/api/errors";
import { getMe, getServerApi } from "@/lib/api/server";
import { loginHref } from "@/lib/auth";
import { requestTime } from "@/lib/requestTime";
import { ReviewForm } from "./ReviewForm";

type Params = Promise<{ id: string }>;
type SP = Promise<Record<string, string | string[] | undefined>>;

export const metadata: Metadata = { title: "Write a review" };

export default function WriteReviewPage({ params, searchParams }: { params: Params; searchParams: SP }) {
  return (
    <Container>
      <Suspense
        fallback={
          <Stack gap={5} style={{ paddingTop: "var(--space-6)" }}>
            <LoadingNote />
            <Placeholder height={64} radius="control" />
            <Placeholder height={240} radius="control" />
          </Stack>
        }
      >
        <WriteReview params={params} searchParams={searchParams} />
      </Suspense>
    </Container>
  );
}

async function WriteReview({ params, searchParams }: { params: Params; searchParams: SP }) {
  const { id } = await params;
  const editParam = (await searchParams).edit;
  const editId = typeof editParam === "string" && editParam ? editParam : null;
  const api = await getServerApi();
  const me = await getMe(api);
  if (!me) redirect(loginHref(`/places/${id}/review${editId ? `?edit=${encodeURIComponent(editId)}` : ""}`));

  const place = await api.place(id).catch((e) => {
    if (isApiError(e) && (e.status === 404 || e.status === 422)) notFound();
    throw e;
  });

  const crumbs = (
    <Crumbs items={[{ label: place.name, href: `/places/${place.id}` }, { label: editId ? "Edit your review" : "Write a review" }]} />
  );

  // Expired students can browse and save, but not post or edit.
  if (me.status === "expired") {
    return (
      <>
        {crumbs}
        <Stack gap={5} style={{ marginTop: "var(--space-4)" }}>
          <PageHead title={`${editId ? "Edit your review of" : "Review"} ${place.name}`} size="title" />
          <ExpiredAlert accessUntil={me.access_until} />
        </Stack>
      </>
    );
  }

  if (me.status !== "verified") {
    const pending = me.status === "pending_email" || me.status === "pending_school";
    return (
      <>
        {crumbs}
        <EmptyState
          icon="school"
          headingLevel={1}
          title={
            me.status === "pending_email"
              ? "Confirm your school email first"
              : me.status === "pending_school"
                ? "Your school is being reviewed"
                : "You can't post reviews yet"
          }
          action={pending ? <Button href="/signup">{me.status === "pending_email" ? "Enter your code" : "See your status"}</Button> : <Button href="/me">Go to your profile</Button>}
        >
          {me.status === "pending_email"
            ? "Enter the 6-digit code we emailed you, then come back to post."
            : me.status === "pending_school"
              ? "We'll email you as soon as it's approved, usually within a day."
              : "Only verified students can post reviews on Corner."}
        </EmptyState>
      </>
    );
  }

  if (editId) {
    const review = await api.review(editId).catch((e) => {
      if (isApiError(e) && (e.status === 404 || e.status === 422)) return null;
      throw e;
    });
    if (!review || !review.is_mine || review.place_id !== place.id) {
      return (
        <>
          {crumbs}
          <EmptyState
            icon="pen"
            headingLevel={1}
            title="We couldn't find that review"
            action={<Button href={`/places/${place.id}`}>Back to {place.name}</Button>}
          >
            Only the person who wrote a review can edit it.
          </EmptyState>
        </>
      );
    }
    return (
      <>
        {crumbs}
        <ReviewForm
          place={{ id: place.id, name: place.name, hasDeal: !!place.student_discount }}
          author={{ displayName: me.display_name, schoolName: review.author.school_name }}
          existing={review}
          renderedAt={await requestTime()}
        />
      </>
    );
  }

  if (place.status !== "active") {
    return (
      <>
        {crumbs}
        <EmptyState
          icon="pin"
          headingLevel={1}
          title="This place isn't open for reviews yet"
          action={<Button href={`/places/${place.id}`}>Back to {place.name}</Button>}
        >
          New places can be reviewed once an admin approves them, usually within a day.
        </EmptyState>
      </>
    );
  }

  if (place.viewer_review_id) {
    return (
      <>
        {crumbs}
        <EmptyState
          icon="pen"
          headingLevel={1}
          title={`You've already reviewed ${place.name}`}
          action={<Button href={`/places/${place.id}#review-${place.viewer_review_id}`}>See your review</Button>}
        >
          Each student can post one review per place.
        </EmptyState>
      </>
    );
  }

  return (
    <>
      {crumbs}
      <ReviewForm
        place={{ id: place.id, name: place.name, hasDeal: !!place.student_discount }}
        author={{ displayName: me.display_name, schoolName: me.school?.name ?? "" }}
      />
    </>
  );
}

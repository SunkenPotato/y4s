import type { Metadata } from "next";
import { notFound, redirect } from "next/navigation";
import { Suspense } from "react";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Container, Crumbs, LoadingNote, Placeholder, Stack } from "@/components/Layout/Layout";
import { isApiError } from "@/lib/api/errors";
import { getMe, getServerApi } from "@/lib/api/server";
import { loginHref } from "@/lib/auth";
import { ReviewForm } from "./ReviewForm";

type Params = Promise<{ id: string }>;

export const metadata: Metadata = { title: "Write a review" };

export default function WriteReviewPage({ params }: { params: Params }) {
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
        <WriteReview params={params} />
      </Suspense>
    </Container>
  );
}

async function WriteReview({ params }: { params: Params }) {
  const { id } = await params;
  const api = await getServerApi();
  const me = await getMe(api);
  if (!me) redirect(loginHref(`/places/${id}/review`));

  const place = await api.place(id).catch((e) => {
    if (isApiError(e) && (e.status === 404 || e.status === 422)) notFound();
    throw e;
  });

  const crumbs = <Crumbs items={[{ label: place.name, href: `/places/${place.id}` }, { label: "Write a review" }]} />;

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

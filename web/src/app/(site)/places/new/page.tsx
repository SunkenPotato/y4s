import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Container, LoadingNote, Placeholder, Stack } from "@/components/Layout/Layout";
import { getMe } from "@/lib/api/server";
import { loginHref } from "@/lib/auth";
import { NewPlaceForm } from "./NewPlaceForm";

export const metadata: Metadata = { title: "Add a place" };

type SP = Promise<Record<string, string | string[] | undefined>>;

export default function NewPlacePage({ searchParams }: { searchParams: SP }) {
  return (
    <Container>
      <Suspense
        fallback={
          <Stack gap={5} style={{ paddingTop: "var(--space-6)" }}>
            <LoadingNote />
            <Placeholder height={64} radius="control" />
            <Placeholder height={320} radius="control" />
          </Stack>
        }
      >
        <NewPlace searchParams={searchParams} />
      </Suspense>
    </Container>
  );
}

async function NewPlace({ searchParams }: { searchParams: SP }) {
  const sp = await searchParams;
  const me = await getMe();
  if (!me) redirect(loginHref("/places/new"));
  if (me.status !== "verified") {
    return (
      <EmptyState
        icon="school"
        headingLevel={1}
        title="Verify your school email to add places"
        action={<Button href="/signup">Finish verifying</Button>}
      >
        Only verified students can add places to Corner.
      </EmptyState>
    );
  }
  const name = typeof sp.name === "string" ? sp.name.slice(0, 120) : "";
  return <NewPlaceForm initialName={name} school={me.school} />;
}

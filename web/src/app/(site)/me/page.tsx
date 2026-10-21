import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Container, LoadingNote, Placeholder, Stack } from "@/components/Layout/Layout";
import { getMe } from "@/lib/api/server";
import { loginHref } from "@/lib/auth";
import { Profile } from "./Profile";

export const metadata: Metadata = { title: "Your profile" };

export default function MePage() {
  return (
    <Container narrow>
      <Suspense
        fallback={
          <Stack gap={5} style={{ paddingTop: "var(--space-6)" }}>
            <LoadingNote />
            <Placeholder height={64} radius="control" />
            <Placeholder height={240} radius="control" />
          </Stack>
        }
      >
        <MeContent />
      </Suspense>
    </Container>
  );
}

async function MeContent() {
  const me = await getMe();
  if (!me) redirect(loginHref("/me"));
  return <Profile initialMe={me} />;
}

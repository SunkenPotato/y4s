import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Container, LoadingNote } from "@/components/Layout/Layout";
import { getMe } from "@/lib/api/server";
import { nextStepFor, safeNext } from "@/lib/auth";
import { LoginForm } from "./LoginForm";

export const metadata: Metadata = { title: "Log in" };

type SP = Promise<Record<string, string | string[] | undefined>>;

export default function LoginPage({ searchParams }: { searchParams: SP }) {
  return (
    <Container narrow>
      <Suspense fallback={<LoadingNote />}>
        <Login searchParams={searchParams} />
      </Suspense>
    </Container>
  );
}

async function Login({ searchParams }: { searchParams: SP }) {
  const next = safeNext((await searchParams).next);
  const me = await getMe().catch(() => null);
  if (me) redirect(nextStepFor(me, next));
  return <LoginForm next={next} />;
}

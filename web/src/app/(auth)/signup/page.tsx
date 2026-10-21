import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Container, LoadingNote } from "@/components/Layout/Layout";
import { getMe } from "@/lib/api/server";
import { SignupFlow } from "./SignupFlow";

export const metadata: Metadata = { title: "Sign up" };

export default function SignupPage() {
  return (
    <Container narrow>
      <Suspense fallback={<LoadingNote />}>
        <Signup />
      </Suspense>
    </Container>
  );
}

async function Signup() {
  const me = await getMe().catch(() => null);
  if (me?.status === "verified") redirect("/");
  return <SignupFlow initialMe={me} />;
}

import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Container, LoadingNote } from "@/components/Layout/Layout";
import { getMe } from "@/lib/api/server";
import { loginHref } from "@/lib/auth";
import { ChangeSchoolFlow } from "./ChangeSchoolFlow";

export const metadata: Metadata = { title: "Change school" };

export default function ChangeSchoolPage() {
  return (
    <Container narrow>
      <Suspense fallback={<LoadingNote />}>
        <ChangeSchool />
      </Suspense>
    </Container>
  );
}

async function ChangeSchool() {
  const me = await getMe();
  if (!me) redirect(loginHref("/me/school"));
  // Only verified or expired students can move schools; everyone else is still signing up.
  if (me.status !== "verified" && me.status !== "expired") redirect(me.status === "suspended" ? "/me" : "/signup");
  return <ChangeSchoolFlow initialMe={me} />;
}

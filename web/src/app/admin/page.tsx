import type { Metadata } from "next";
import { redirect } from "next/navigation";
import { Suspense } from "react";
import { Container, LoadingNote, Placeholder, Stack } from "@/components/Layout/Layout";
import { getMe, getServerApi } from "@/lib/api/server";
import { loginHref } from "@/lib/auth";
import { AdminConsole } from "./AdminConsole";

export const metadata: Metadata = { title: "Admin" };

export default function AdminPage() {
  return (
    <Container>
      <Suspense
        fallback={
          <Stack gap={4} style={{ paddingTop: "var(--space-5)" }}>
            <LoadingNote />
            <Placeholder height={56} radius="control" />
            <Placeholder height={120} />
            <Placeholder height={120} />
          </Stack>
        }
      >
        <Admin />
      </Suspense>
    </Container>
  );
}

async function Admin() {
  const api = await getServerApi();
  const me = await getMe(api);
  if (!me) redirect(loginHref("/admin"));
  if (me.role !== "admin" || me.status !== "verified") redirect("/");

  const [pending, approved, rejected, places, reports] = await Promise.all([
    api.adminDomains("pending"),
    api.adminDomains("approved"),
    api.adminDomains("rejected"),
    api.adminPlaces(),
    api.adminReports("open"),
  ]);
  const decided = [...approved, ...rejected]
    .sort((a, b) => (b.decided_at ?? "").localeCompare(a.decided_at ?? ""))
    .slice(0, 6);

  return <AdminConsole pending={pending} decided={decided} places={places} reports={reports} />;
}

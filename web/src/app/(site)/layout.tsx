import { Suspense, type ReactNode } from "react";
import { Header } from "@/components/Header/Header";
import { Container } from "@/components/Layout/Layout";
import { TabBar, TabBarView } from "@/components/TabBar/TabBar";
import { getMe } from "@/lib/api/server";

async function SiteHeader() {
  const me = await getMe().catch(() => null);
  return <Header me={me} />;
}

export default function SiteLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <Container className="site-header">
        <Suspense fallback={<Header me={undefined} />}>
          <SiteHeader />
        </Suspense>
      </Container>
      <main id="main">{children}</main>
      <Suspense fallback={<TabBarView pathname={null} />}>
        <TabBar />
      </Suspense>
    </>
  );
}

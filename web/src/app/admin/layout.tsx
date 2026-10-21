import type { ReactNode } from "react";
import { Header } from "@/components/Header/Header";
import { Container } from "@/components/Layout/Layout";

export default function AdminLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <Container className="site-header">
        <Header variant="admin" />
      </Container>
      <main id="main">{children}</main>
    </>
  );
}

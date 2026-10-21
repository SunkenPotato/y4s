import type { ReactNode } from "react";
import { Header } from "@/components/Header/Header";
import { Container } from "@/components/Layout/Layout";

export default function AuthLayout({ children }: { children: ReactNode }) {
  return (
    <>
      <Container narrow className="site-header">
        <Header variant="minimal" />
      </Container>
      <main id="main">{children}</main>
    </>
  );
}

import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Header } from "@/components/Header/Header";
import { Container } from "@/components/Layout/Layout";

export default function NotFound() {
  return (
    <Container>
      <Header variant="minimal" />
      <main id="main">
        <EmptyState
          icon="pin"
          headingLevel={1}
          title="We couldn't find that page"
          action={<Button href="/">Back to places</Button>}
        >
          The link may be old, or the place was removed.
        </EmptyState>
      </main>
    </Container>
  );
}

"use client";

import { useEffect } from "react";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Container } from "@/components/Layout/Layout";

export default function SiteError({ error, retry }: { error: unknown; retry: () => void }) {
  useEffect(() => {
    console.error(error);
  }, [error]);

  return (
    <Container>
      <EmptyState
        icon="warning"
        headingLevel={1}
        title="Couldn't load this page"
        action={
          <Button onClick={() => retry()}>Try again</Button>
        }
      >
        Corner didn&apos;t answer. Check your connection, then try again.
      </EmptyState>
    </Container>
  );
}

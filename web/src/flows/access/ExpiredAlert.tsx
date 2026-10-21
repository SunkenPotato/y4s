import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { monthYear } from "@/lib/format";

/**
 * Shown wherever an expired user would post, and for access_expired errors:
 * "Your Corner access ended {Mon YYYY}. Verify a new school email to keep posting."
 */
export function ExpiredAlert({ accessUntil, className }: { accessUntil: string; className?: string }) {
  return (
    <Alert
      tone="warning"
      className={className}
      title={`Your Corner access ended ${monthYear(accessUntil)}.`}
      action={
        <Button href="/me/school" size={36} variant="secondary">
          Change school
        </Button>
      }
    >
      Verify a new school email to keep posting.
    </Alert>
  );
}

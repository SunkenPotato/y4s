"use client";

import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";
import { Muted, PageHead, Stack } from "@/components/Layout/Layout";
import { StepList } from "@/components/StepList/StepList";
import type { Me } from "@/lib/api/types";
import { emailDomain } from "@/lib/format";
import styles from "./flow.module.css";

/** Email confirmed; waiting for an admin to match the domain to the school. */
export function PendingStep({ me, onRepick }: { me: Me; onRepick?: () => void }) {
  const school = me.school?.name ?? "your school";
  return (
    <div className={styles.screen}>
      <Stack gap={3}>
        <span>
          <Badge tone="pending">Pending review</Badge>
        </span>
        <PageHead title={`We're checking ${school}`} size="title">
          Your email is confirmed. Once we&apos;ve matched <span className={styles.strong}>@{emailDomain(me.email)}</span> to your
          school, you can post. We&apos;ll email you.
        </PageHead>
      </Stack>
      <StepList
        label="Verification progress"
        steps={[
          { title: "Email confirmed", detail: me.email, state: "done" },
          { title: "School check", detail: "Usually within a day", state: "current" },
          { title: "Write reviews and add places", state: "todo" },
        ]}
      />
      <Stack gap={2}>
        <Button href="/" block>
          Browse places meanwhile
        </Button>
        {onRepick ? (
          <Button variant="ghost" block onClick={onRepick}>
            Picked the wrong school?
          </Button>
        ) : null}
      </Stack>
      <p className={`${styles.footer} ${styles.center}`}>
        <Muted small as="span">
          Corner is for students 13 and older.
        </Muted>
      </p>
    </div>
  );
}

/** The domain was rejected for the chosen school. */
export function RejectedStep({ me, onStartOver, onRepick }: { me: Me; onStartOver: () => void; onRepick?: () => void }) {
  const school = me.school?.name ?? "your school";
  return (
    <div className={styles.screen}>
      <Stack gap={3}>
        <span>
          <Badge tone="rejected">Not approved</Badge>
        </span>
        <PageHead title={`We couldn't match your email to ${school}`} size="title">
          <span className={styles.strong}>@{emailDomain(me.email)}</span> wasn&apos;t approved as a school address. If you have
          another school email, sign up with that one.
        </PageHead>
      </Stack>
      <Stack gap={2}>
        <Button block onClick={onStartOver}>
          Sign up with another email
        </Button>
        {onRepick ? (
          <Button variant="ghost" block onClick={onRepick}>
            Picked the wrong school?
          </Button>
        ) : null}
        <Button href="/" block variant="ghost">
          Browse places
        </Button>
      </Stack>
    </div>
  );
}

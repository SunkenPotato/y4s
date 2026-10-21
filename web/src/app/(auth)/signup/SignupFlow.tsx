"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Button } from "@/components/Button/Button";
import { PageHead } from "@/components/Layout/Layout";
import { useToast } from "@/components/Toast/Toast";
import { AccountStep } from "@/flows/verification/AccountStep";
import { CodeStep } from "@/flows/verification/CodeStep";
import { SchoolStep } from "@/flows/verification/SchoolStep";
import { PendingStep, RejectedStep } from "@/flows/verification/StatusSteps";
import styles from "@/flows/verification/flow.module.css";
import { api } from "@/lib/api/client";
import type { Me } from "@/lib/api/types";
import { emailDomain } from "@/lib/format";

type Step = "account" | "school" | "code" | "pending" | "rejected" | "suspended" | "done";

function stepFor(me: Me | null): Step {
  if (!me) return "account";
  switch (me.status) {
    case "pending_email":
      return me.needs_school ? "school" : "code";
    case "pending_school":
      return me.needs_school ? "school" : "pending";
    case "rejected":
      return "rejected";
    case "suspended":
      return "suspended";
    default:
      return "done";
  }
}

/** Signup is driven by Me.status and needs_school: account → (school) → code → verified or pending. */
export function SignupFlow({ initialMe }: { initialMe: Me | null }) {
  const router = useRouter();
  const toast = useToast();
  const [me, setMe] = useState<Me | null>(initialMe);
  const [hadSchoolStep, setHadSchoolStep] = useState(!!initialMe?.needs_school);
  // "Picked the wrong school?" sends the user back to the picker.
  const [repicking, setRepicking] = useState(false);
  const step = repicking ? "school" : stepFor(me);
  const total = hadSchoolStep ? 3 : 2;

  function update(user: Me | null) {
    if (user?.needs_school) setHadSchoolStep(true);
    setRepicking(false);
    setMe(user);
    if (user?.status === "verified") {
      toast.show({ message: "You're in. Welcome to Corner." });
      router.replace("/");
      router.refresh();
    }
  }

  async function startOver() {
    try {
      await api.logout();
    } catch {
      /* the session may already be gone */
    }
    setHadSchoolStep(false);
    setRepicking(false);
    setMe(null);
    router.refresh();
  }

  switch (step) {
    case "account":
      return <AccountStep mode="signup" stepLabel={`Step 1 of ${total}`} onDone={update} />;
    case "school":
      return (
        <SchoolStep
          domain={emailDomain(me!.email)}
          stepLabel={repicking ? undefined : `Step 2 of ${total}`}
          choose={api.chooseSchool}
          onDone={update}
          onBack={repicking ? () => setRepicking(false) : undefined}
          intro={
            repicking ? (
              <>
                Pick the school <span className={styles.strong}>@{emailDomain(me!.email)}</span> belongs to. We&apos;ll check it again.
              </>
            ) : undefined
          }
        />
      );
    case "code":
      return (
        <CodeStep
          email={me!.email}
          stepLabel={`Step ${total} of ${total}`}
          verify={api.verify}
          resend={api.resend}
          refresh={api.me}
          onDone={update}
        />
      );
    case "pending":
      return <PendingStep me={me!} onRepick={() => setRepicking(true)} />;
    case "rejected":
      return <RejectedStep me={me!} onStartOver={startOver} onRepick={() => setRepicking(true)} />;
    case "suspended":
      return (
        <div className={styles.screen}>
          <PageHead title="This account is suspended" size="title">
            You can still browse places, but you can&apos;t post.
          </PageHead>
          <Button href="/" block>
            Browse places
          </Button>
        </div>
      );
    default:
      return (
        <div className={styles.screen}>
          <PageHead title="You're verified" size="title">
            Taking you to places near your school.
          </PageHead>
        </div>
      );
  }
}

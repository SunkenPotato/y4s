"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { TextLink } from "@/components/Layout/Layout";
import { useToast } from "@/components/Toast/Toast";
import { AccountStep } from "@/flows/verification/AccountStep";
import { CodeStep } from "@/flows/verification/CodeStep";
import { SchoolStep } from "@/flows/verification/SchoolStep";
import { PendingStep } from "@/flows/verification/StatusSteps";
import { api, isApiError } from "@/lib/api/client";
import type { Me } from "@/lib/api/types";
import { emailDomain } from "@/lib/format";

/**
 * Change school: new school email + start month → (school picker, if needs_school) → code.
 * Uses the same step components as signup, bound to the /api/me/school-change endpoints.
 */
export function ChangeSchoolFlow({ initialMe }: { initialMe: Me }) {
  const router = useRouter();
  const toast = useToast();
  const [me, setMe] = useState(initialMe);
  const [hadSchoolStep, setHadSchoolStep] = useState(!!initialMe.pending_school_change?.needs_school);
  const change = me.pending_school_change;
  const total = hadSchoolStep ? 3 : 2;

  function update(user: Me) {
    if (user.pending_school_change?.needs_school) setHadSchoolStep(true);
    setMe(user);
    if (!user.pending_school_change && user.status === "verified") {
      toast.show({ message: `You're verified at ${user.school?.name ?? "your new school"}` });
      router.replace("/me");
      router.refresh();
    }
  }

  async function cancel() {
    if (change) {
      try {
        await api.cancelSchoolChange();
      } catch {
        /* nothing in progress */
      }
    }
    router.push("/me");
    router.refresh();
  }

  // The change applied but the new domain still needs an admin: same screen as signup.
  if (!change && me.status === "pending_school") {
    return <PendingStep me={me} />;
  }

  const cancelLink = <TextLink onClick={cancel}>Cancel</TextLink>;

  if (!change) {
    return <AccountStep mode="change" stepLabel={`Step 1 of ${total}`} onDone={update} topAction={<TextLink href="/me">Cancel</TextLink>} />;
  }

  if (change.needs_school) {
    return (
      <SchoolStep
        domain={emailDomain(change.email)}
        stepLabel={`Step 2 of ${total}`}
        choose={api.chooseChangeSchool}
        onDone={update}
        topAction={cancelLink}
      />
    );
  }

  async function verify(code: string) {
    try {
      return await api.verifySchoolChange(code);
    } catch (e) {
      // The API wants a school picked first: reload and show the picker.
      if (isApiError(e) && e.code === "school_choice_required") update(await api.me());
      throw e;
    }
  }

  return (
    <CodeStep
      email={change.email}
      stepLabel={`Step ${total} of ${total}`}
      verify={verify}
      resend={api.resendSchoolChange}
      onDone={update}
      topAction={cancelLink}
    />
  );
}

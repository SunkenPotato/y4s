"use client";

import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { PageHead } from "@/components/Layout/Layout";
import { useToast } from "@/components/Toast/Toast";
import { isApiError, type ApiError } from "@/lib/api/client";
import type { Me } from "@/lib/api/types";
import { WhySchoolEmail } from "./AccountStep";
import styles from "./flow.module.css";

const RESEND_SECONDS = 60;

function codeAlert(err: ApiError): { title: string; text: string } {
  switch (err.code) {
    case "invalid_code": {
      const left = err.attemptsLeft;
      if (left === 0) return { title: "That code didn't work", text: "No attempts left. Send a new code and try again." };
      return {
        title: "That code didn't work",
        text: `Check the latest email and try again.${left !== undefined ? ` ${left} ${left === 1 ? "attempt" : "attempts"} left.` : ""}`,
      };
    }
    case "too_many_attempts":
      return { title: "Too many tries", text: "Send a new code and enter it within 15 minutes." };
    case "code_expired":
      return { title: "That code expired", text: "Send a new code and enter it within 15 minutes." };
    case "rate_limited":
      return { title: "Too many tries", text: err.message };
    default:
      return { title: "Couldn't verify", text: err.message };
  }
}

interface CodeStepProps {
  /** Where the code was sent. */
  email: string;
  stepLabel?: string;
  /** POST /api/auth/verify or /api/me/school-change/verify. */
  verify: (code: string) => Promise<Me>;
  /** POST /api/auth/resend or /api/me/school-change/resend. */
  resend: () => Promise<void>;
  /** Called after already_verified to fetch the current user. */
  refresh?: () => Promise<Me>;
  onDone: (me: Me) => void;
  topAction?: ReactNode;
}

/** Single 6-digit code field with a resend countdown and attempts-left messaging. */
export function CodeStep({ email, stepLabel, verify, resend, refresh, onDone, topAction }: CodeStepProps) {
  const toast = useToast();
  const [code, setCode] = useState("");
  const [fieldError, setFieldError] = useState<string | null>(null);
  const [alert, setAlert] = useState<{ title: string; text: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const [resending, setResending] = useState(false);
  const [until, setUntil] = useState(() => Date.now() + RESEND_SECONDS * 1000);
  const [now, setNow] = useState(() => Date.now());
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const t = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(t);
  }, []);

  const left = Math.max(0, Math.ceil((until - now) / 1000));
  const countdown = `${Math.floor(left / 60)}:${String(left % 60).padStart(2, "0")}`;

  async function submit(e: FormEvent) {
    e.preventDefault();
    setAlert(null);
    if (!/^\d{6}$/.test(code)) {
      setFieldError("Enter the 6 digits from the email.");
      inputRef.current?.focus();
      return;
    }
    setFieldError(null);
    setLoading(true);
    try {
      onDone(await verify(code));
    } catch (err) {
      setLoading(false);
      if (!isApiError(err)) return setAlert({ title: "Couldn't verify", text: "Check your connection and try again." });
      if (err.code === "already_verified" && refresh) {
        try {
          onDone(await refresh());
        } catch {
          /* fall through */
        }
        return;
      }
      if (err.code === "validation") {
        setFieldError(err.fields.code ?? "Enter the 6 digits from the email.");
        return;
      }
      setAlert(codeAlert(err));
      setCode("");
      inputRef.current?.focus();
    }
  }

  async function sendAgain() {
    setResending(true);
    setAlert(null);
    try {
      await resend();
      setUntil(Date.now() + RESEND_SECONDS * 1000);
      setNow(Date.now());
      toast.show({ message: `New code sent to ${email}` });
    } catch (err) {
      if (isApiError(err) && err.code === "rate_limited") {
        setUntil(Date.now() + (err.retryAfter ?? RESEND_SECONDS) * 1000);
        setNow(Date.now());
      } else {
        setAlert({ title: "Couldn't send a new code", text: isApiError(err) ? err.message : "Try again in a moment." });
      }
    } finally {
      setResending(false);
    }
  }

  return (
    <div className={styles.screen}>
      <div className={styles.topRow}>
        {topAction ?? <span />}
        {stepLabel ? <span className={styles.step}>{stepLabel}</span> : null}
      </div>
      <PageHead title="Check your school inbox" size="title">
        We sent a 6-digit code to <span className={styles.strong}>{email}</span>. It works for 15 minutes.
      </PageHead>
      <form onSubmit={submit} noValidate className={styles.form}>
        {alert ? (
          <Alert tone="error" title={alert.title}>
            {alert.text}
          </Alert>
        ) : null}
        <Field
          label="Code"
          error={fieldError}
          helper={left > 0 ? `Paste works too. You can request a new code in ${countdown}.` : "Paste works too."}
        >
          <Input
            ref={inputRef}
            variant="code"
            inputMode="numeric"
            autoComplete="one-time-code"
            pattern="[0-9]*"
            maxLength={6}
            value={code}
            onChange={(e) => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
          />
        </Field>
        <Button type="submit" block loading={loading} loadingLabel="Verifying…">
          Verify
        </Button>
        {left === 0 ? (
          <Button variant="ghost" block onClick={sendAgain} loading={resending} loadingLabel="Sending…">
            Send a new code
          </Button>
        ) : null}
      </form>
      <div className={styles.footer}>
        <WhySchoolEmail />
      </div>
    </div>
  );
}


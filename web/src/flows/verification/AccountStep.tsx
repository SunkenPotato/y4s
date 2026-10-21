"use client";

import { useState, type FormEvent, type ReactNode } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { Muted, PageHead, TextLink } from "@/components/Layout/Layout";
import { Tile } from "@/components/Tile/Tile";
import { api, isApiError } from "@/lib/api/client";
import type { Me } from "@/lib/api/types";
import { ENROLLMENT_TOO_OLD, enrollmentError, toEnrolledOn } from "@/lib/enrollment";
import { EnrollmentField } from "./EnrollmentField";
import styles from "./flow.module.css";

interface AccountStepProps {
  /** "signup": the full account form. "change": only the new school email and start month. */
  mode: "signup" | "change";
  stepLabel: string;
  onDone: (me: Me) => void;
  /** Shown above the title, like a Cancel link in the change flow. */
  topAction?: ReactNode;
}

type Form = { email: string; password: string; first_name: string; last_name: string; birth_year: string; month: string; year: string };

/** Step 1 of signup, and the first step of changing school. */
export function AccountStep({ mode, stepLabel, onDone, topAction }: AccountStepProps) {
  const change = mode === "change";
  const [form, setForm] = useState<Form>({ email: "", password: "", first_name: "", last_name: "", birth_year: "", month: "", year: "" });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<{ title: string; text?: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const set = (k: keyof Form) => (e: { target: { value: string } }) => setForm((f) => ({ ...f, [k]: e.target.value }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    const local: Record<string, string> = {};
    if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(form.email.trim()))
      local.email = change ? "Enter your new school email, like name@nyu.edu." : "Enter your school email, like name@ucla.edu.";
    if (!change) {
      if (form.password.length < 10) local.password = "Use at least 10 characters.";
      if (!form.first_name.trim()) local.first_name = "Enter your first name.";
      if (!form.last_name.trim()) local.last_name = "Enter your last name.";
      if (!/^\d{4}$/.test(form.birth_year)) local.birth_year = "Enter the year you were born, like 2006.";
    }
    const enrolled = enrollmentError(form.month, form.year, new Date());
    if (enrolled) local.enrolled_on = enrolled;
    setErrors(local);
    setFormError(null);
    if (Object.keys(local).length) return;

    setLoading(true);
    const enrolled_on = toEnrolledOn(form.month, form.year);
    try {
      const me = change
        ? await api.startSchoolChange({ email: form.email.trim(), enrolled_on })
        : await api.signup({
            email: form.email.trim(),
            password: form.password,
            first_name: form.first_name.trim(),
            last_name: form.last_name.trim(),
            birth_year: Number(form.birth_year),
            enrolled_on,
          });
      onDone(me);
    } catch (err) {
      setLoading(false);
      const title = change ? "Couldn't start the change" : "Couldn't create your account";
      if (!isApiError(err)) return setFormError({ title, text: "Check your connection and try again." });
      if (err.code === "validation") setErrors(err.fields);
      else if (err.code === "not_school_email") setErrors({ email: err.fields.email ?? err.message });
      else if (err.code === "too_young") setErrors({ birth_year: err.fields.birth_year ?? err.message });
      else if (err.code === "enrollment_too_old") setErrors({ enrolled_on: err.message || ENROLLMENT_TOO_OLD });
      else if (err.code === "email_taken") setErrors({ email: err.message });
      else setFormError({ title, text: err.message });
    }
  }

  return (
    <div className={styles.screen}>
      <div className={styles.topRow}>
        {topAction ?? <span />}
        <span className={styles.step}>{stepLabel}</span>
      </div>
      {change ? (
        <PageHead title="Change school" size="title">
          Verify your new school email. Your reviews keep the school they were posted under, and your saved places come with you.
        </PageHead>
      ) : (
        <PageHead title="Join Corner" size="title">
          Use your school email. College and high-school addresses both work.
        </PageHead>
      )}
      <form onSubmit={submit} noValidate className={styles.form}>
        {formError ? (
          <Alert tone="error" title={formError.title}>
            {formError.text}
          </Alert>
        ) : null}
        <Field
          label={change ? "New school email" : "School email"}
          helper={change ? "Your current email keeps working until you confirm this one." : "We'll send a 6-digit code to confirm it."}
          error={errors.email}
        >
          <Input type="email" autoComplete="email" inputMode="email" value={form.email} onChange={set("email")} />
        </Field>
        {!change ? (
          <>
            <Field label="Password" helper="At least 10 characters." error={errors.password}>
              <Input type="password" autoComplete="new-password" value={form.password} onChange={set("password")} />
            </Field>
            <div className={styles.pair}>
              <Field label="First name" error={errors.first_name}>
                <Input autoComplete="given-name" maxLength={40} value={form.first_name} onChange={set("first_name")} />
              </Field>
              <Field label="Last name" helper="Others see only your last initial." error={errors.last_name}>
                <Input autoComplete="family-name" maxLength={40} value={form.last_name} onChange={set("last_name")} />
              </Field>
            </div>
            <Field label="Birth year" helper="Corner is for students 13 and older." error={errors.birth_year}>
              <Input inputMode="numeric" autoComplete="bday-year" maxLength={4} value={form.birth_year} onChange={set("birth_year")} />
            </Field>
          </>
        ) : null}
        <EnrollmentField
          month={form.month}
          year={form.year}
          error={errors.enrolled_on}
          onChange={({ month, year }) => {
            setForm((f) => ({ ...f, month, year }));
            if (errors.enrolled_on) setErrors(({ enrolled_on: _e, ...rest }) => rest);
          }}
        />
        <Button type="submit" block loading={loading} loadingLabel={change ? "Sending code…" : "Creating account…"}>
          {change ? "Send code" : "Create account"}
        </Button>
      </form>
      {!change ? (
        <Muted>
          Already have an account? <TextLink href="/login">Log in</TextLink>
        </Muted>
      ) : null}
      <div className={styles.footer}>
        <WhySchoolEmail />
      </div>
    </div>
  );
}

export function WhySchoolEmail() {
  return (
    <Tile title="Why a school email?">
      <Muted>It&apos;s how we know every review comes from a student. Others only see your first name, last initial and school.</Muted>
    </Tile>
  );
}

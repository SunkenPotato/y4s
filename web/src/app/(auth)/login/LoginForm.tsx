"use client";

import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { Muted, PageHead, TextLink } from "@/components/Layout/Layout";
import { api, isApiError } from "@/lib/api/client";
import { nextStepFor } from "@/lib/auth";
import styles from "../auth.module.css";

export function LoginForm({ next }: { next: string }) {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [errors, setErrors] = useState<{ email?: string; password?: string }>({});
  const [formError, setFormError] = useState<{ title: string; text?: string } | null>(null);
  const [loading, setLoading] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    const local: typeof errors = {};
    if (!email.trim()) local.email = "Enter your school email.";
    if (!password) local.password = "Enter your password.";
    setErrors(local);
    setFormError(null);
    if (Object.keys(local).length) return;
    setLoading(true);
    try {
      const me = await api.login(email.trim(), password);
      router.replace(nextStepFor(me, next));
      router.refresh();
    } catch (err) {
      setLoading(false);
      if (!isApiError(err)) return setFormError({ title: "Couldn't log in", text: "Check your connection and try again." });
      if (err.code === "validation") setErrors(err.fields);
      else if (err.code === "invalid_credentials") setFormError({ title: "That didn't match", text: err.message });
      else if (err.code === "suspended") setFormError({ title: "Account suspended", text: err.message });
      else setFormError({ title: "Couldn't log in", text: err.message });
    }
  }

  return (
    <div className={styles.screen}>
      <PageHead title="Log in" size="title">
        Use the school email you signed up with.
      </PageHead>
      <form onSubmit={submit} noValidate className={styles.form}>
        {formError ? (
          <Alert tone="error" title={formError.title}>
            {formError.text}
          </Alert>
        ) : null}
        <Field label="School email" error={errors.email}>
          <Input type="email" autoComplete="email" inputMode="email" value={email} onChange={(e) => setEmail(e.target.value)} />
        </Field>
        <Field label="Password" error={errors.password}>
          <Input type="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} />
        </Field>
        <Button type="submit" block loading={loading} loadingLabel="Logging in…">
          Log in
        </Button>
      </form>
      <Muted>
        New to Corner? <TextLink href="/signup">Sign up with your school email</TextLink>
      </Muted>
    </div>
  );
}

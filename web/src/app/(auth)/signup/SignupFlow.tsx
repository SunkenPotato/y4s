"use client";

import { useRouter } from "next/navigation";
import { useEffect, useRef, useState, type FormEvent } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";
import { ChoiceList } from "@/components/ChoiceList/ChoiceList";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { Muted, PageHead, Stack, TextLink } from "@/components/Layout/Layout";
import { SearchField } from "@/components/SearchField/SearchField";
import { Segmented } from "@/components/Segmented/Segmented";
import { StepList } from "@/components/StepList/StepList";
import { Tile } from "@/components/Tile/Tile";
import { useToast } from "@/components/Toast/Toast";
import { api, isApiError, type ApiError } from "@/lib/api/client";
import type { Me, School, SchoolKind } from "@/lib/api/types";
import { emailDomain, schoolKind, schoolPlace } from "@/lib/format";
import styles from "../auth.module.css";

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
  const step = stepFor(me);
  const total = hadSchoolStep ? 3 : 2;

  function update(user: Me | null) {
    if (user?.needs_school) setHadSchoolStep(true);
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
    setMe(null);
    router.refresh();
  }

  switch (step) {
    case "account":
      return <AccountStep total={total} onDone={update} />;
    case "school":
      return <SchoolStep me={me!} stepLabel={`Step 2 of ${total}`} onDone={update} />;
    case "code":
      return <CodeStep me={me!} stepLabel={`Step ${total} of ${total}`} onDone={update} />;
    case "pending":
      return <PendingStep me={me!} />;
    case "rejected":
      return <RejectedStep me={me!} onStartOver={startOver} />;
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

/* Step 1: account */

function AccountStep({ total, onDone }: { total: number; onDone: (me: Me) => void }) {
  const [form, setForm] = useState({ email: "", password: "", first_name: "", last_name: "", birth_year: "" });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<{ title: string; text?: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const set = (k: keyof typeof form) => (e: { target: { value: string } }) => setForm((f) => ({ ...f, [k]: e.target.value }));

  async function submit(e: FormEvent) {
    e.preventDefault();
    const local: Record<string, string> = {};
    if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(form.email.trim())) local.email = "Enter your school email, like name@ucla.edu.";
    if (form.password.length < 10) local.password = "Use at least 10 characters.";
    if (!form.first_name.trim()) local.first_name = "Enter your first name.";
    if (!form.last_name.trim()) local.last_name = "Enter your last name.";
    if (!/^\d{4}$/.test(form.birth_year)) local.birth_year = "Enter the year you were born, like 2006.";
    setErrors(local);
    setFormError(null);
    if (Object.keys(local).length) return;

    setLoading(true);
    try {
      const me = await api.signup({
        email: form.email.trim(),
        password: form.password,
        first_name: form.first_name.trim(),
        last_name: form.last_name.trim(),
        birth_year: Number(form.birth_year),
      });
      onDone(me);
    } catch (err) {
      setLoading(false);
      if (!isApiError(err)) return setFormError({ title: "Couldn't create your account", text: "Check your connection and try again." });
      if (err.code === "validation") setErrors(err.fields);
      else if (err.code === "not_school_email") setErrors({ email: err.fields.email ?? err.message });
      else if (err.code === "too_young") setErrors({ birth_year: err.fields.birth_year ?? err.message });
      else if (err.code === "email_taken") setErrors({ email: err.message });
      else setFormError({ title: "Couldn't create your account", text: err.message });
    }
  }

  return (
    <div className={styles.screen}>
      <div className={styles.topRow}>
        <span />
        <span className={styles.step}>Step 1 of {total}</span>
      </div>
      <PageHead title="Join Corner" size="title">
        Use your school email. College and high-school addresses both work.
      </PageHead>
      <form onSubmit={submit} noValidate className={styles.form}>
        {formError ? (
          <Alert tone="error" title={formError.title}>
            {formError.text}
          </Alert>
        ) : null}
        <Field label="School email" helper="We'll send a 6-digit code to confirm it." error={errors.email}>
          <Input type="email" autoComplete="email" inputMode="email" value={form.email} onChange={set("email")} />
        </Field>
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
        <Button type="submit" block loading={loading} loadingLabel="Creating account…">
          Create account
        </Button>
      </form>
      <Muted>
        Already have an account? <TextLink href="/login">Log in</TextLink>
      </Muted>
      <div className={styles.footer}>
        <WhySchoolEmail />
      </div>
    </div>
  );
}

function WhySchoolEmail() {
  return (
    <Tile title="Why a school email?">
      <Muted>It&apos;s how we know every review comes from a student. Others only see your first name, last initial and school.</Muted>
    </Tile>
  );
}

/* Step 2: school (only when needs_school) */

const KINDS = [
  { value: "high_school", label: "High school" },
  { value: "university", label: "College" },
] as const;

function SchoolStep({ me, stepLabel, onDone }: { me: Me; stepLabel: string; onDone: (me: Me) => void }) {
  const domain = emailDomain(me.email);
  const [q, setQ] = useState("");
  const [results, setResults] = useState<School[]>([]);
  const [searching, setSearching] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [newSchool, setNewSchool] = useState({ name: "", kind: "high_school" as SchoolKind, city: "", state: "" });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const query = q.trim();
    if (query.length < 2) return;
    const t = window.setTimeout(async () => {
      setSearching(true);
      try {
        setResults(await api.schools(query, { limit: 8 }));
      } catch {
        setResults([]);
      } finally {
        setSearching(false);
      }
    }, 250);
    return () => window.clearTimeout(t);
  }, [q]);

  const shown = q.trim().length < 2 ? [] : results;

  async function submit(e: FormEvent) {
    e.preventDefault();
    setFormError(null);
    setErrors({});
    if (!adding && !selected) {
      setErrors({ school_id: "Pick your school, or add it if it isn't listed." });
      return;
    }
    if (adding) {
      const local: Record<string, string> = {};
      if (!newSchool.name.trim()) local["new_school.name"] = "Enter your school's full name.";
      if (!newSchool.city.trim()) local["new_school.city"] = "Enter the city.";
      if (!newSchool.state.trim()) local["new_school.state"] = "Enter the state, like CA.";
      if (Object.keys(local).length) return setErrors(local);
    }
    setLoading(true);
    try {
      const user = await api.chooseSchool(
        adding
          ? { new_school: { name: newSchool.name.trim(), kind: newSchool.kind, city: newSchool.city.trim(), state: newSchool.state.trim().toUpperCase() } }
          : { school_id: selected! },
      );
      onDone(user);
    } catch (err) {
      setLoading(false);
      if (isApiError(err) && err.code === "validation" && Object.keys(err.fields).length) {
        // Accept both "new_school.name" and "name" style keys.
        const mapped: Record<string, string> = {};
        for (const [k, v] of Object.entries(err.fields)) mapped[k.startsWith("new_school.") || k === "school_id" ? k : `new_school.${k}`] = v;
        setErrors(mapped);
      } else {
        setFormError(isApiError(err) ? err.message : "Couldn't save your school. Try again.");
      }
    }
  }

  return (
    <div className={styles.screen}>
      <div className={styles.topRow}>
        <span />
        <span className={styles.step}>{stepLabel}</span>
      </div>
      <PageHead title="Which school is this?" size="title">
        We haven&apos;t seen <span className={styles.strong}>@{domain}</span> yet. Pick your school and we&apos;ll check the
        address belongs to it.
      </PageHead>
      <form onSubmit={submit} noValidate className={styles.form}>
        {formError ? <Alert tone="error">{formError}</Alert> : null}
        {!adding ? (
          <Stack gap={2}>
            <SearchField
              label="School"
              showLabel
              value={q}
              onChange={(e) => {
                setQ(e.target.value);
                setSelected(null);
              }}
              placeholder="Start typing your school's name"
              autoComplete="off"
            />
            <p role="status" className="visually-hidden">
              {searching ? "Searching…" : q.trim().length >= 2 ? `${shown.length} schools found` : ""}
            </p>
            <ChoiceList
              name="school"
              label="Matching schools"
              choices={shown.map((s) => ({ value: s.id, label: s.name, detail: `${schoolPlace(s)} · ${schoolKind(s.kind)}` }))}
              value={selected}
              onChange={(v) => {
                setSelected(v);
                setErrors({});
              }}
              error={errors.school_id}
              footer={<TextLink onClick={() => setAdding(true)}>Not listed? Add your school</TextLink>}
            />
          </Stack>
        ) : (
          <Stack gap={4}>
            <Field label="School name" error={errors["new_school.name"]}>
              <Input value={newSchool.name} onChange={(e) => setNewSchool((s) => ({ ...s, name: e.target.value }))} autoComplete="organization" />
            </Field>
            <fieldset>
              <legend className="visually-hidden">Kind of school</legend>
              <Segmented label="Kind of school" options={KINDS} value={newSchool.kind} onChange={(kind) => setNewSchool((s) => ({ ...s, kind }))} />
            </fieldset>
            <div className={styles.pair}>
              <Field label="City" error={errors["new_school.city"]}>
                <Input value={newSchool.city} onChange={(e) => setNewSchool((s) => ({ ...s, city: e.target.value }))} autoComplete="address-level2" />
              </Field>
              <Field label="State" error={errors["new_school.state"]}>
                <Input value={newSchool.state} maxLength={2} onChange={(e) => setNewSchool((s) => ({ ...s, state: e.target.value }))} autoComplete="address-level1" />
              </Field>
            </div>
            <TextLink onClick={() => setAdding(false)}>Search the list instead</TextLink>
          </Stack>
        )}
        <Button type="submit" block loading={loading} loadingLabel="Requesting…">
          Request access
        </Button>
      </form>
      <div className={styles.footer}>
        <Tile title="What happens next">
          <Muted>
            Someone on our team checks that this email domain belongs to your school, usually within a day. After that, everyone at
            your school signs up instantly.
          </Muted>
        </Tile>
      </div>
    </div>
  );
}

/* Step 3: code */

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

function CodeStep({ me, stepLabel, onDone }: { me: Me; stepLabel: string; onDone: (me: Me) => void }) {
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
      onDone(await api.verify(code));
    } catch (err) {
      setLoading(false);
      if (!isApiError(err)) return setAlert({ title: "Couldn't verify", text: "Check your connection and try again." });
      if (err.code === "already_verified") {
        try {
          onDone(await api.me());
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

  async function resend() {
    setResending(true);
    setAlert(null);
    try {
      await api.resend();
      setUntil(Date.now() + RESEND_SECONDS * 1000);
      setNow(Date.now());
      toast.show({ message: `New code sent to ${me.email}` });
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
        <span />
        <span className={styles.step}>{stepLabel}</span>
      </div>
      <PageHead title="Check your school inbox" size="title">
        We sent a 6-digit code to <span className={styles.strong}>{me.email}</span>. It works for 15 minutes.
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
          <Button variant="ghost" block onClick={resend} loading={resending} loadingLabel="Sending…">
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

/* Step 4: pending or rejected */

function PendingStep({ me }: { me: Me }) {
  const school = me.school?.name ?? "your school";
  return (
    <div className={styles.screen}>
      <Stack gap={3}>
        <span>
          <Badge tone="pending">Pending review</Badge>
        </span>
        <PageHead title={`We're checking ${school}`} size="title">
          Your email is confirmed. Once we&apos;ve matched <span className={styles.strong}>@{emailDomain(me.email)}</span> to
          your school, you can post. We&apos;ll email you.
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
      <Button href="/" block>
        Browse places meanwhile
      </Button>
      <p className={`${styles.footer} ${styles.center}`}>
        <Muted small as="span">
          Corner is for students 13 and older.
        </Muted>
      </p>
    </div>
  );
}

function RejectedStep({ me, onStartOver }: { me: Me; onStartOver: () => void }) {
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
        <Button href="/" block variant="ghost">
          Browse places
        </Button>
      </Stack>
    </div>
  );
}

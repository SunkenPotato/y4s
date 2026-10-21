"use client";

import { useRouter } from "next/navigation";
import { useState, type FormEvent } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Avatar } from "@/components/Avatar/Avatar";
import { Badge, type BadgeTone } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { Muted, PageHead, Row, Section, Stack } from "@/components/Layout/Layout";
import { Tile } from "@/components/Tile/Tile";
import { useToast } from "@/components/Toast/Toast";
import { ExpiredAlert } from "@/flows/access/ExpiredAlert";
import { api, isApiError } from "@/lib/api/client";
import type { Me, Page, Place, UserStatus } from "@/lib/api/types";
import { monthYear, schoolKind, schoolPlace } from "@/lib/format";
import styles from "./profile.module.css";
import { SavedPlaces } from "./SavedPlaces";

const STATUS: Record<UserStatus, { label: string; tone: BadgeTone; text: string }> = {
  verified: { label: "Verified", tone: "verified", text: "You can write reviews and add places." },
  expired: { label: "Access ended", tone: "rejected", text: "You can still browse and save places." },
  pending_email: { label: "Confirm your email", tone: "pending", text: "Enter the code we emailed you to finish signing up." },
  pending_school: { label: "Pending review", tone: "pending", text: "We're checking your school. We'll email you, usually within a day." },
  rejected: { label: "Not approved", tone: "rejected", text: "Your email domain wasn't approved for this school." },
  suspended: { label: "Suspended", tone: "rejected", text: "You can browse, but you can't post." },
};

export function Profile({ initialMe, saved }: { initialMe: Me; saved: Page<Place> | null }) {
  const router = useRouter();
  const toast = useToast();
  const [me, setMe] = useState(initialMe);
  const [first, setFirst] = useState(initialMe.first_name);
  const [last, setLast] = useState(initialMe.last_name);
  const [errors, setErrors] = useState<{ first_name?: string; last_name?: string }>({});
  const [formError, setFormError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [canceling, setCanceling] = useState(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const status = STATUS[me.status];
  const dirty = first.trim() !== me.first_name || last.trim() !== me.last_name;
  const change = me.pending_school_change;
  const canChangeSchool = me.status === "verified" || me.status === "expired";

  async function save(e: FormEvent) {
    e.preventDefault();
    const local: typeof errors = {};
    if (!first.trim()) local.first_name = "Enter your first name.";
    if (!last.trim()) local.last_name = "Enter your last name.";
    setErrors(local);
    setFormError(null);
    if (Object.keys(local).length) return;
    setSaving(true);
    try {
      const user = await api.updateMe({ first_name: first.trim(), last_name: last.trim() });
      setMe(user);
      setFirst(user.first_name);
      setLast(user.last_name);
      toast.show({ message: `Saved. Reviews now show ${user.display_name}` });
      router.refresh();
    } catch (err) {
      if (isApiError(err) && err.code === "validation") setErrors(err.fields);
      else setFormError(isApiError(err) ? err.message : "Couldn't save. Try again.");
    } finally {
      setSaving(false);
    }
  }

  async function cancelChange() {
    setCanceling(true);
    try {
      await api.cancelSchoolChange();
      setMe((m) => ({ ...m, pending_school_change: null }));
      toast.show({ message: "School change canceled" });
      router.refresh();
    } catch (err) {
      toast.show({ message: isApiError(err) ? err.message : "Couldn't cancel. Try again." });
    } finally {
      setCanceling(false);
    }
  }

  async function logout() {
    setLoggingOut(true);
    try {
      await api.logout();
    } catch {
      /* already logged out */
    }
    router.replace("/");
    router.refresh();
  }

  return (
    <Stack gap={6} className={styles.screen}>
      <Row gap={4}>
        <Avatar name={me.display_name} size={44} />
        <PageHead title={me.display_name} size="title">
          {me.email}
        </PageHead>
      </Row>

      {me.status === "expired" ? (
        <ExpiredAlert accessUntil={me.access_until} />
      ) : (
        <Tile title="Status">
          <Stack gap={2}>
            <span>
              <Badge tone={status.tone}>{status.label}</Badge>
            </span>
            <Muted>{status.text}</Muted>
            {me.status === "pending_email" || me.status === "pending_school" || me.status === "rejected" ? (
              <span>
                <Button href="/signup" size={44} variant="secondary">
                  {me.status === "pending_email" ? "Enter your code" : "See your status"}
                </Button>
              </span>
            ) : null}
          </Stack>
        </Tile>
      )}

      {change ? (
        <Tile tone="warn" title={`Confirming ${change.email}`}>
          <Stack gap={3}>
            <p>
              {change.needs_school
                ? "Pick your new school, then enter the code we emailed to that address."
                : `Enter the code we emailed to finish moving to ${change.school?.name ?? "your new school"}.`}{" "}
              Until then, your current school email keeps working.
            </p>
            <Row gap={2}>
              <Button href="/me/school" size={44}>
                Continue
              </Button>
              <Button variant="ghost" size={44} onClick={cancelChange} loading={canceling} loadingLabel="Canceling…">
                Cancel
              </Button>
            </Row>
          </Stack>
        </Tile>
      ) : null}

      <Section title="Saved places" id="saved">
        <SavedPlaces initial={saved} />
      </Section>

      <Section title="School" id="school">
        <Stack gap={3}>
          {me.school ? (
            <div>
              <p className={styles.schoolName}>{me.school.name}</p>
              <Muted small>
                {schoolPlace(me.school)} · {schoolKind(me.school.kind)} ·{" "}
                {me.status === "expired" ? `Access ended ${monthYear(me.access_until)}` : `Access until ${monthYear(me.access_until)}`}
              </Muted>
            </div>
          ) : (
            <Muted>Pick your school to finish signing up.</Muted>
          )}
          {canChangeSchool ? (
            <>
              <Muted>
                Moving to a new school? Verify your new school email. Reviews you&apos;ve already posted keep the school they were
                posted under.
              </Muted>
              {!change ? (
                <span>
                  <Button href="/me/school" variant="secondary">
                    Change school
                  </Button>
                </span>
              ) : null}
            </>
          ) : null}
        </Stack>
      </Section>

      <Section title="Your name">
        <form onSubmit={save} noValidate>
          <Stack gap={4}>
            {formError ? <Alert tone="error">{formError}</Alert> : null}
            <div className={styles.pair}>
              <Field label="First name" error={errors.first_name}>
                <Input value={first} maxLength={40} autoComplete="given-name" onChange={(e) => setFirst(e.target.value)} />
              </Field>
              <Field label="Last name" helper="Others see only your last initial." error={errors.last_name}>
                <Input value={last} maxLength={40} autoComplete="family-name" onChange={(e) => setLast(e.target.value)} />
              </Field>
            </div>
            <span>
              <Button type="submit" variant="secondary" loading={saving} loadingLabel="Saving…" disabled={!dirty}>
                Save name
              </Button>
            </span>
          </Stack>
        </form>
      </Section>

      <div>
        <Button variant="danger" onClick={logout} loading={loggingOut} loadingLabel="Logging out…">
          Log out
        </Button>
      </div>
    </Stack>
  );
}

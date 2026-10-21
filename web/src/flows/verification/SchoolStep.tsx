"use client";

import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { ChoiceList } from "@/components/ChoiceList/ChoiceList";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { Muted, PageHead, Stack, TextLink } from "@/components/Layout/Layout";
import { SearchField } from "@/components/SearchField/SearchField";
import { Segmented } from "@/components/Segmented/Segmented";
import { Tile } from "@/components/Tile/Tile";
import { api, isApiError } from "@/lib/api/client";
import type { ChooseSchoolInput, Me, School, SchoolKind } from "@/lib/api/types";
import { schoolKind, schoolPlace } from "@/lib/format";
import styles from "./flow.module.css";

const KINDS = [
  { value: "high_school", label: "High school" },
  { value: "university", label: "College" },
] as const;

interface SchoolStepProps {
  /** The email domain being matched, without "@". */
  domain: string;
  stepLabel?: string;
  /** POST /api/auth/school or /api/me/school-change/school. */
  choose: (input: ChooseSchoolInput) => Promise<Me>;
  onDone: (me: Me) => void;
  /** Shows a Back link (used when re-picking from the pending screen). */
  onBack?: () => void;
  /** Shown top-left when there's no Back link, like Cancel in the change flow. */
  topAction?: ReactNode;
  intro?: ReactNode;
}

/** Pick the school an unknown email domain belongs to, or add a new one. */
export function SchoolStep({ domain, stepLabel, choose, onDone, onBack, topAction, intro }: SchoolStepProps) {
  const [q, setQ] = useState("");
  const [results, setResults] = useState<School[]>([]);
  const [searching, setSearching] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);
  const [newSchool, setNewSchool] = useState({ name: "", kind: "high_school" as SchoolKind, city: "", state: "" });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [formError, setFormError] = useState<{ title: string; text: string } | null>(null);
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
      const user = await choose(
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
      } else if (isApiError(err) && err.code === "school_locked") {
        setFormError({ title: "This school can't be changed", text: err.message });
      } else {
        setFormError({ title: "Couldn't save your school", text: isApiError(err) ? err.message : "Check your connection and try again." });
      }
    }
  }

  return (
    <div className={styles.screen}>
      <div className={styles.topRow}>
        {onBack ? <TextLink onClick={onBack}>Back</TextLink> : (topAction ?? <span />)}
        {stepLabel ? <span className={styles.step}>{stepLabel}</span> : null}
      </div>
      <PageHead title="Which school is this?" size="title">
        {intro ?? (
          <>
            We haven&apos;t seen <span className={styles.strong}>@{domain}</span> yet. Pick your school and we&apos;ll check the address
            belongs to it.
          </>
        )}
      </PageHead>
      <form onSubmit={submit} noValidate className={styles.form}>
        {formError ? (
          <Alert tone="error" title={formError.title}>
            {formError.text}
          </Alert>
        ) : null}
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
            Someone on our team checks that this email domain belongs to your school, usually within a day. After that, everyone at your
            school signs up instantly.
          </Muted>
        </Tile>
      </div>
    </div>
  );
}

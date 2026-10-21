"use client";

import { useState, type FormEvent } from "react";
import { Alert } from "@/components/Alert/Alert";
import { Button } from "@/components/Button/Button";
import { Field } from "@/components/Field/Field";
import { Input } from "@/components/Input/Input";
import { Aside, Columns, MainColumn, PageHead, Row, Stack, TextLink } from "@/components/Layout/Layout";
import { Segmented } from "@/components/Segmented/Segmented";
import { Select } from "@/components/Select/Select";
import { Tile } from "@/components/Tile/Tile";
import { ExpiredAlert } from "@/flows/access/ExpiredAlert";
import { api, isApiError } from "@/lib/api/client";
import { CATEGORIES, type Category, type PlaceDetail, type PlaceKind, type PriceLevel, type SchoolSummary } from "@/lib/api/types";
import { CATEGORY_SINGULAR } from "@/lib/format";
import { HoursEditor, type HoursRowDraft } from "./HoursEditor";
import styles from "./new.module.css";

const KINDS = [
  { value: "business", label: "Near campus" },
  { value: "facility", label: "On campus" },
] as const;

type Errors = Partial<Record<"name" | "category" | "subcategory" | "address" | "price_level" | "student_discount", string>>;

export function NewPlaceForm({ initialName, school }: { initialName: string; school: SchoolSummary | null }) {
  const [kind, setKind] = useState<PlaceKind>("business");
  const [name, setName] = useState(initialName);
  const [category, setCategory] = useState<Category>("food");
  const [subcategory, setSubcategory] = useState("");
  const [address, setAddress] = useState("");
  const [priceLevel, setPriceLevel] = useState("");
  const [deal, setDeal] = useState("");
  const [hours, setHours] = useState<HoursRowDraft[]>([]);
  const [hoursErrors, setHoursErrors] = useState<Record<number, string>>({});
  const [hoursError, setHoursError] = useState<string | null>(null);
  const [errors, setErrors] = useState<Errors>({});
  const [formError, setFormError] = useState<{ title: string; text?: string; existingId?: string } | null>(null);
  const [expiredUntil, setExpiredUntil] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [created, setCreated] = useState<PlaceDetail | null>(null);

  async function submit(e: FormEvent) {
    e.preventDefault();
    const local: Errors = {};
    if (!name.trim()) local.name = "Enter the place's name.";
    if (!address.trim()) local.address = "Enter the street address, like 1062 Gayley Ave, Los Angeles.";
    const rowErrors: Record<number, string> = {};
    for (const r of hours) if (!r.time.trim()) rowErrors[r.key] = "Enter the hours, like 11 am – 10 pm.";
    setErrors(local);
    setHoursErrors(rowErrors);
    setHoursError(null);
    setFormError(null);
    if (Object.keys(local).length || Object.keys(rowErrors).length) return;

    setSaving(true);
    try {
      const place = await api.createPlace({
        kind,
        name: name.trim(),
        category,
        subcategory: subcategory.trim() || null,
        address: address.trim(),
        school_id: kind === "facility" ? (school?.id ?? null) : null,
        price_level: priceLevel ? (Number(priceLevel) as PriceLevel) : null,
        student_discount: deal.trim() || null,
        hours: hours.length ? hours.map((r) => ({ days: r.days, time: r.time.trim() })) : null,
      });
      setCreated(place);
    } catch (err) {
      if (!isApiError(err)) {
        setFormError({ title: "Couldn't add this place", text: "Check your connection and try again." });
      } else if (err.code === "validation" || err.code === "geocode_failed") {
        setErrors(err.fields as Errors);
        // hours errors may come as "hours" or "hours.0.time".
        const rowErr: Record<number, string> = {};
        for (const [k, v] of Object.entries(err.fields)) {
          const m = k.match(/^hours\.(\d+)/);
          if (m && hours[Number(m[1])]) rowErr[hours[Number(m[1])].key] = v;
          else if (k === "hours") setHoursError(v);
        }
        setHoursErrors(rowErr);
        if (!Object.keys(err.fields).length) setFormError({ title: "Check the details", text: err.message });
      } else if (err.code === "access_expired") {
        const me = await api.me().catch(() => null);
        if (me?.access_until) setExpiredUntil(me.access_until);
        else setFormError({ title: "You can't add places right now", text: err.message });
      } else if (err.code === "duplicate_place") {
        const existingId = typeof err.extras.existing_id === "string" ? err.extras.existing_id : undefined;
        setFormError({ title: "This place is already on Corner", text: err.message, existingId });
      } else {
        setFormError({ title: "Couldn't add this place", text: err.message });
      }
    } finally {
      setSaving(false);
    }
  }

  if (created) {
    return (
      <div className={styles.layout}>
        <Stack gap={5}>
          <PageHead title="Place sent for review" size="title">
            We&apos;ll check {created.name} and add it, usually within a day.
          </PageHead>
          <Alert tone="warning" title="Pending review">
            Only you can see it until an admin approves it.
          </Alert>
          <Row gap={2}>
            <Button href={`/places/${created.id}`}>See {created.name}</Button>
            <Button
              variant="secondary"
              onClick={() => {
                setCreated(null);
                setName("");
                setSubcategory("");
                setAddress("");
                setPriceLevel("");
                setDeal("");
                setHours([]);
              }}
            >
              Add another place
            </Button>
          </Row>
        </Stack>
      </div>
    );
  }

  return (
    <Columns gap={7} className={styles.layout}>
      <MainColumn>
        <form onSubmit={submit} noValidate className={styles.form}>
          <PageHead title="Add a place" size="title">
            New places show up after a quick check, usually within a day.
          </PageHead>

          {expiredUntil ? <ExpiredAlert accessUntil={expiredUntil} /> : null}
          {formError ? (
            <Alert
              tone="error"
              title={formError.title}
              action={
                formError.existingId ? (
                  <Button size={36} variant="secondary" href={`/places/${formError.existingId}`}>
                    Go to it
                  </Button>
                ) : undefined
              }
            >
              {formError.text}
            </Alert>
          ) : null}

          <fieldset>
            <legend className={styles.legend}>Where is it?</legend>
            <Segmented label="Where is it?" options={KINDS} value={kind} onChange={setKind} />
          </fieldset>

          <Field label="Name" error={errors.name}>
            <Input value={name} onChange={(e) => setName(e.target.value)} autoComplete="off" maxLength={120} />
          </Field>

          <div className={styles.pair}>
            <Field label="Category" error={errors.category}>
              <Select value={category} onChange={(e) => setCategory(e.target.value as Category)}>
                {CATEGORIES.map((c) => (
                  <option key={c} value={c}>
                    {CATEGORY_SINGULAR[c]}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label="Type (optional)" helper="Like Ramen, Boba or Printing." error={errors.subcategory}>
              <Input value={subcategory} onChange={(e) => setSubcategory(e.target.value)} maxLength={40} />
            </Field>
          </div>

          <Field
            label="Address"
            helper={kind === "facility" ? "Building name and campus address." : "Street and city, like 1062 Gayley Ave, Los Angeles."}
            error={errors.address}
          >
            <Input value={address} onChange={(e) => setAddress(e.target.value)} autoComplete="street-address" />
          </Field>

          <div className={styles.pair}>
            <Field label="Price" error={errors.price_level}>
              <Select value={priceLevel} onChange={(e) => setPriceLevel(e.target.value)}>
                <option value="">Not sure</option>
                <option value="1">$</option>
                <option value="2">$$</option>
                <option value="3">$$$</option>
                <option value="4">$$$$</option>
              </Select>
            </Field>
            <Field label="Student deal (optional)" helper="Like 15% off Mon–Thu." error={errors.student_discount}>
              <Input value={deal} onChange={(e) => setDeal(e.target.value)} maxLength={80} />
            </Field>
          </div>

          <HoursEditor
            rows={hours}
            onChange={(rows) => {
              setHours(rows);
              setHoursError(null);
              setHoursErrors((errs) => Object.fromEntries(Object.entries(errs).filter(([k]) => rows.some((r) => String(r.key) === k))));
            }}
            errors={hoursErrors}
            error={hoursError}
          />

          <Row gap={3}>
            <Button type="submit" loading={saving} loadingLabel="Adding…">
              Add place
            </Button>
            <TextLink href="/">Cancel</TextLink>
          </Row>
        </form>
      </MainColumn>
      <Aside label="Tips">
        <Tile title="What belongs on Corner">
          <p>Places students actually use: food, coffee, study spots, gyms and services near {school?.name ?? "your school"}.</p>
        </Tile>
      </Aside>
    </Columns>
  );
}

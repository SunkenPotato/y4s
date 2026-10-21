"use client";

import { Button } from "@/components/Button/Button";
import { Field } from "@/components/Field/Field";
import { Icon } from "@/components/Icon/Icon";
import { IconButton } from "@/components/IconButton/IconButton";
import { Input } from "@/components/Input/Input";
import { Stack } from "@/components/Layout/Layout";
import { Select } from "@/components/Select/Select";
import styles from "./new.module.css";

export interface HoursRowDraft {
  key: number;
  days: string;
  time: string;
}

/** Day labels in the same style as the API's hours ("Mon – Thu"). */
export const DAY_OPTIONS = ["Mon – Fri", "Mon – Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] as const;

const nextDays = (rows: HoursRowDraft[]) => {
  const last = rows[rows.length - 1]?.days;
  if (!last) return "Mon – Fri";
  if (last === "Mon – Fri") return "Sat";
  if (last === "Sat") return "Sun";
  return "Mon";
};

let keySeq = 1;

interface HoursEditorProps {
  rows: HoursRowDraft[];
  onChange: (rows: HoursRowDraft[]) => void;
  /** Per-row time errors, keyed by row key, plus a general message. */
  errors: Record<number, string>;
  error?: string | null;
}

/** Optional opening hours: collapsed behind "Add opening hours"; rows of days + free-text time. */
export function HoursEditor({ rows, onChange, errors, error }: HoursEditorProps) {
  const add = () => onChange([...rows, { key: keySeq++, days: nextDays(rows), time: "" }]);
  const update = (key: number, patch: Partial<HoursRowDraft>) =>
    onChange(rows.map((r) => (r.key === key ? { ...r, ...patch } : r)));
  const remove = (key: number) => onChange(rows.filter((r) => r.key !== key));

  if (rows.length === 0) {
    return (
      <div>
        <Button variant="ghost" onClick={add} icon={<Icon name="plus" size={18} />} className={styles.flushLeft}>
          Add opening hours
        </Button>
      </div>
    );
  }

  return (
    <fieldset className={styles.hours}>
      <legend className={styles.legend}>Opening hours (optional)</legend>
      <Stack gap={3}>
        {rows.map((row, i) => (
          <div key={row.key} className={styles.hoursRow}>
            <Field label={`Days, row ${i + 1}`} hideLabel className={styles.days}>
              <Select value={row.days} onChange={(e) => update(row.key, { days: e.target.value })}>
                {DAY_OPTIONS.map((d) => (
                  <option key={d} value={d}>
                    {d}
                  </option>
                ))}
              </Select>
            </Field>
            <Field label={`Hours, row ${i + 1}`} hideLabel error={errors[row.key]} className={styles.time}>
              <Input
                value={row.time}
                placeholder="11 am – 10 pm"
                maxLength={40}
                onChange={(e) => update(row.key, { time: e.target.value })}
              />
            </Field>
            <IconButton aria-label={`Remove ${row.days} hours`} variant="ghost" onClick={() => remove(row.key)}>
              <Icon name="close" />
            </IconButton>
          </div>
        ))}
        {error ? <p className={styles.error}>{error}</p> : null}
        <div>
          <Button variant="ghost" size={44} onClick={add} icon={<Icon name="plus" size={18} />} className={styles.flushLeft}>
            Add row
          </Button>
        </div>
      </Stack>
    </fieldset>
  );
}

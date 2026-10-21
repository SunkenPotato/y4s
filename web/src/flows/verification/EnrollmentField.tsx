"use client";

import { useId, useState } from "react";
import { Field } from "@/components/Field/Field";
import { Select } from "@/components/Select/Select";
import { enrollmentYears } from "@/lib/enrollment";
import { MONTH_NAMES } from "@/lib/format";
import styles from "./flow.module.css";

interface EnrollmentFieldProps {
  month: string;
  year: string;
  onChange: (value: { month: string; year: string }) => void;
  error?: string | null;
}

/** "When did you start at this school?" as a month Select and a year Select. */
export function EnrollmentField({ month, year, onChange, error }: EnrollmentFieldProps) {
  const id = useId().replace(/:/g, "");
  const [years] = useState(() => enrollmentYears(new Date()));
  const messageId = `${id}-message`;
  return (
    <fieldset className={styles.enrollment}>
      <legend className={styles.legend}>When did you start at this school?</legend>
      <div className={styles.selects}>
        <Field label="Month" hideLabel>
          <Select
            value={month}
            aria-invalid={!!error}
            aria-describedby={messageId}
            onChange={(e) => onChange({ month: e.target.value, year })}
          >
            <option value="">Month</option>
            {MONTH_NAMES.map((name, i) => (
              <option key={name} value={String(i + 1)}>
                {name}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Year" hideLabel>
          <Select
            value={year}
            aria-invalid={!!error}
            aria-describedby={messageId}
            onChange={(e) => onChange({ month, year: e.target.value })}
          >
            <option value="">Year</option>
            {years.map((y) => (
              <option key={y} value={String(y)}>
                {y}
              </option>
            ))}
          </Select>
        </Field>
      </div>
      <p id={messageId} className={error ? styles.error : styles.message}>
        {error ?? "Your access lasts 5 years from this month."}
      </p>
    </fieldset>
  );
}

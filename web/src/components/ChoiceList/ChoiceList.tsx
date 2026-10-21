import type { ReactNode } from "react";
import styles from "./ChoiceList.module.css";

export interface Choice {
  value: string;
  label: string;
  detail?: string;
}

interface ChoiceListProps {
  name: string;
  label: string;
  choices: Choice[];
  value: string | null;
  onChange: (value: string) => void;
  /** A trailing row, like "Not listed? Add your school". */
  footer?: ReactNode;
  describedBy?: string;
  /** Shown under the list and linked with aria-describedby. */
  error?: string | null;
}

/** A radiogroup of native radios on a surface tile; the checked row turns bg. Arrow keys work natively. */
export function ChoiceList({ name, label, choices, value, onChange, footer, describedBy, error }: ChoiceListProps) {
  const errorId = `${name}-error`;
  const described = [describedBy, error ? errorId : null].filter(Boolean).join(" ") || undefined;
  return (
    <div className={styles.wrap}>
    <div role="radiogroup" aria-label={label} aria-describedby={described} aria-invalid={error ? true : undefined} className={styles.list}>
      {choices.map((c) => (
        <label key={c.value} className={[styles.row, value === c.value && styles.checked].filter(Boolean).join(" ")}>
          <input
            type="radio"
            name={name}
            value={c.value}
            checked={value === c.value}
            onChange={() => onChange(c.value)}
            className={styles.radio}
          />
          <span className={styles.text}>
            <span className={styles.label}>{c.label}</span>
            {c.detail ? <span className={styles.detail}>{c.detail}</span> : null}
          </span>
        </label>
      ))}
      {footer ? <div className={styles.footer}>{footer}</div> : null}
    </div>
    {error ? (
      <p id={errorId} className={styles.error}>
        {error}
      </p>
    ) : null}
    </div>
  );
}

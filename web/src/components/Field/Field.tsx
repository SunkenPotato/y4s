"use client";

import { createContext, useContext, useId, type ReactNode } from "react";
import styles from "./Field.module.css";

interface FieldContextValue {
  id: string;
  invalid: boolean;
  describedBy: string | undefined;
  disabled: boolean;
}

const FieldContext = createContext<FieldContextValue | null>(null);

/** Inputs call this to pick up the id, aria-invalid and aria-describedby from their Field. */
export function useFieldControl(props: {
  id?: string;
  "aria-invalid"?: boolean | "true" | "false" | "grammar" | "spelling";
  "aria-describedby"?: string;
  disabled?: boolean;
}) {
  const field = useContext(FieldContext);
  const invalid = props["aria-invalid"] === true || props["aria-invalid"] === "true" || !!field?.invalid;
  const describedBy = [field?.describedBy, props["aria-describedby"]].filter(Boolean).join(" ") || undefined;
  return {
    id: props.id ?? field?.id,
    invalid,
    disabled: props.disabled ?? field?.disabled ?? false,
    aria: {
      "aria-invalid": invalid || undefined,
      "aria-describedby": describedBy,
    },
  };
}

interface FieldProps {
  label: ReactNode;
  /** Shown between the label and the control. */
  description?: ReactNode;
  /** Shown below the control. Replaced by `error` when there is one. */
  helper?: ReactNode;
  error?: string | null;
  /** Right-aligned in the message row, like a character counter. */
  counter?: ReactNode;
  /** "large" is the 18px bold label used on the review form. */
  labelSize?: "default" | "large";
  hideLabel?: boolean;
  disabled?: boolean;
  id?: string;
  className?: string;
  children: ReactNode;
}

/**
 * Label above, control, then helper text. In the error state the message replaces the
 * helper text and the control gets aria-invalid plus aria-describedby pointing at it.
 */
export function Field({
  label,
  description,
  helper,
  error,
  counter,
  labelSize = "default",
  hideLabel,
  disabled = false,
  id: idProp,
  className,
  children,
}: FieldProps) {
  const autoId = useId();
  const id = idProp ?? `f${autoId.replace(/:/g, "")}`;
  const descriptionId = description ? `${id}-description` : undefined;
  const messageId = error || helper ? `${id}-message` : undefined;
  const describedBy = [descriptionId, messageId].filter(Boolean).join(" ") || undefined;
  const invalid = !!error;

  return (
    <FieldContext.Provider value={{ id, invalid, describedBy, disabled }}>
      <div className={[styles.field, className].filter(Boolean).join(" ")}>
        <label
          htmlFor={id}
          className={[
            styles.label,
            labelSize === "large" && styles.large,
            disabled && styles.disabled,
            hideLabel && "visually-hidden",
          ]
            .filter(Boolean)
            .join(" ")}
        >
          {label}
        </label>
        {description ? (
          <p id={descriptionId} className={styles.description}>
            {description}
          </p>
        ) : null}
        {children}
        {error || helper || counter ? (
          <div className={styles.messageRow}>
            {error ? (
              <span id={messageId} className={styles.error}>
                {error}
              </span>
            ) : helper ? (
              <span id={messageId} className={styles.helper}>
                {helper}
              </span>
            ) : (
              <span />
            )}
            {counter ? <span className={styles.counter}>{counter}</span> : null}
          </div>
        ) : null}
      </div>
    </FieldContext.Provider>
  );
}

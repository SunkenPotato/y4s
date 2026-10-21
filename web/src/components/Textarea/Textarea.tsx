"use client";

import { forwardRef, type TextareaHTMLAttributes } from "react";
import { useFieldControl } from "@/components/Field/Field";
import { controlStyles } from "@/components/Input/Input";
import styles from "./Textarea.module.css";

/** Multi-line Input. Pair it with Field's `counter` and `formatCounter` for the character counter. */
export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  function Textarea({ className, rows = 6, ...props }, ref) {
    const { id, aria, disabled } = useFieldControl(props);
    return (
      <textarea
        ref={ref}
        rows={rows}
        {...props}
        id={id}
        disabled={disabled}
        {...aria}
        className={[controlStyles.control, styles.textarea, className].filter(Boolean).join(" ")}
      />
    );
  },
);

/** "34 / 2,000" */
export function formatCounter(length: number, max: number) {
  return `${length.toLocaleString("en-US")} / ${max.toLocaleString("en-US")}`;
}

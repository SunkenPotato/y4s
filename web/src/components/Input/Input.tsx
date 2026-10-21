"use client";

import { forwardRef, type InputHTMLAttributes } from "react";
import { useFieldControl } from "@/components/Field/Field";
import styles from "./Input.module.css";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  /** "code" is the large one-time-code style. */
  variant?: "default" | "code";
}

/** Surface fill, 12px radius, 48px tall, 2px accent border on focus. Wire it up inside a Field. */
export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { variant = "default", className, ...props },
  ref,
) {
  const { id, aria, disabled } = useFieldControl(props);
  return (
    <input
      ref={ref}
      {...props}
      id={id}
      disabled={disabled}
      {...aria}
      className={[styles.control, variant === "code" && styles.code, className].filter(Boolean).join(" ")}
    />
  );
});

export { styles as controlStyles };

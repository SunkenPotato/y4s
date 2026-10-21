import type { InputHTMLAttributes, ReactNode } from "react";
import styles from "./Checkbox.module.css";

interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  label: ReactNode;
}

/** Native checkbox in the accent color, with a 48px tall label as the hit area. */
export function Checkbox({ label, className, ...props }: CheckboxProps) {
  return (
    <label className={[styles.label, className].filter(Boolean).join(" ")}>
      <input type="checkbox" className={styles.box} {...props} />
      <span>{label}</span>
    </label>
  );
}

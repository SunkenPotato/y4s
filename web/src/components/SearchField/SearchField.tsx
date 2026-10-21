"use client";

import { forwardRef, useId, type InputHTMLAttributes } from "react";
import { Icon } from "@/components/Icon/Icon";
import styles from "./SearchField.module.css";

interface SearchFieldProps extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  /** Accessible name. Shown only when `showLabel` is set. */
  label: string;
  showLabel?: boolean;
}

/** Search input with a leading magnifier, on a surface fill. */
export const SearchField = forwardRef<HTMLInputElement, SearchFieldProps>(function SearchField(
  { label, showLabel = false, className, id: idProp, ...props },
  ref,
) {
  const autoId = useId();
  const id = idProp ?? `s${autoId.replace(/:/g, "")}`;
  return (
    <div className={[styles.root, className].filter(Boolean).join(" ")}>
      <label htmlFor={id} className={showLabel ? styles.label : "visually-hidden"}>
        {label}
      </label>
      <div className={styles.box}>
        <Icon name="search" size={18} className={styles.icon} />
        <input ref={ref} id={id} type="search" enterKeyHint="search" className={styles.input} {...props} />
      </div>
    </div>
  );
});

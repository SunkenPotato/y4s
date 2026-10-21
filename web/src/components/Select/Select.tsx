"use client";

import { forwardRef, type SelectHTMLAttributes } from "react";
import { useFieldControl } from "@/components/Field/Field";
import { Icon } from "@/components/Icon/Icon";
import { controlStyles } from "@/components/Input/Input";
import styles from "./Select.module.css";

/** Native select styled like Input, with a chevron. */
export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(function Select(
  { className, children, ...props },
  ref,
) {
  const { id, aria, disabled } = useFieldControl(props);
  return (
    <div className={styles.wrap}>
      <select
        ref={ref}
        {...props}
        id={id}
        disabled={disabled}
        {...aria}
        className={[controlStyles.control, styles.select, className].filter(Boolean).join(" ")}
      >
        {children}
      </select>
      <Icon name="chevronDown" size={18} className={styles.chevron} />
    </div>
  );
});

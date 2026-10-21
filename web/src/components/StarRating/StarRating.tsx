"use client";

import { useRef, useState, type KeyboardEvent } from "react";
import styles from "./StarRating.module.css";

interface DisplayProps {
  value: number;
  className?: string;
}

/** Read-only stars: an accent ★ string with an aria-label. */
export function Stars({ value, className }: DisplayProps) {
  const full = Math.max(0, Math.min(5, Math.round(value)));
  const label = `${Number.isInteger(value) ? value : value.toFixed(1)} out of 5 stars`;
  return (
    <span role="img" aria-label={label} className={[styles.display, className].filter(Boolean).join(" ")}>
      {"★".repeat(full)}
      {"☆".repeat(5 - full)}
    </span>
  );
}

interface InputProps {
  value: number | null;
  onChange: (value: number) => void;
  label?: string;
  /** id of the element describing the error, when there is one. */
  errorId?: string;
  invalid?: boolean;
  size?: 44 | 48;
  id?: string;
}

const NAMES = ["1 star", "2 stars", "3 stars", "4 stars", "5 stars"];

/**
 * Star input: a radiogroup of 44px (or 48px) stars. Arrow keys move and select,
 * Home/End jump to 1 and 5. Only the checked star (or the first) is in the tab order.
 */
export function StarInput({ value, onChange, label = "Your rating", errorId, invalid, size = 44, id }: InputProps) {
  const refs = useRef<Array<HTMLButtonElement | null>>([]);
  const [hover, setHover] = useState<number | null>(null);
  const shown = hover ?? value ?? 0;

  function select(n: number) {
    onChange(n);
    refs.current[n - 1]?.focus();
  }

  function onKeyDown(e: KeyboardEvent<HTMLButtonElement>, n: number) {
    let next = 0;
    if (e.key === "ArrowRight" || e.key === "ArrowUp") next = Math.min(5, n + 1);
    else if (e.key === "ArrowLeft" || e.key === "ArrowDown") next = Math.max(1, n - 1);
    else if (e.key === "Home") next = 1;
    else if (e.key === "End") next = 5;
    else if (e.key === " " || e.key === "Enter") next = n;
    if (!next) return;
    e.preventDefault();
    select(next);
  }

  return (
    <div
      id={id}
      role="radiogroup"
      aria-label={label}
      aria-describedby={errorId}
      aria-invalid={invalid || undefined}
      className={styles.group}
      onMouseLeave={() => setHover(null)}
    >
      {NAMES.map((name, i) => {
        const n = i + 1;
        const checked = value === n;
        const focusable = value ? checked : n === 1;
        return (
          <button
            key={n}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            role="radio"
            aria-checked={checked}
            aria-label={name}
            tabIndex={focusable ? 0 : -1}
            className={[styles.star, size === 48 && styles.large, n <= shown && styles.filled].filter(Boolean).join(" ")}
            onClick={() => select(n)}
            onKeyDown={(e) => onKeyDown(e, n)}
            onMouseEnter={() => setHover(n)}
          >
            <span aria-hidden="true">★</span>
          </button>
        );
      })}
    </div>
  );
}

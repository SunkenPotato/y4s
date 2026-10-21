"use client";

import { useRef, type KeyboardEvent } from "react";
import styles from "./Segmented.module.css";

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
}

interface SegmentedProps<T extends string> {
  label: string;
  options: readonly SegmentOption<T>[];
  value: T;
  onChange: (value: T) => void;
  /**
   * "buttons" (default): a group of aria-pressed buttons, for sorting.
   * "tabs": a tablist with arrow-key navigation; pass `idPrefix` so panels can use
   * aria-labelledby={`${idPrefix}-tab-${value}`} and id={`${idPrefix}-panel-${value}`}.
   */
  mode?: "buttons" | "tabs";
  idPrefix?: string;
  className?: string;
}

/** Surface track; the selected segment is a bg-colored pill. */
export function Segmented<T extends string>({
  label,
  options,
  value,
  onChange,
  mode = "buttons",
  idPrefix = "seg",
  className,
}: SegmentedProps<T>) {
  const refs = useRef<Array<HTMLButtonElement | null>>([]);
  const tabs = mode === "tabs";

  function onKeyDown(e: KeyboardEvent<HTMLButtonElement>, index: number) {
    if (!tabs) return;
    let next = -1;
    if (e.key === "ArrowRight") next = (index + 1) % options.length;
    else if (e.key === "ArrowLeft") next = (index - 1 + options.length) % options.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = options.length - 1;
    if (next < 0) return;
    e.preventDefault();
    refs.current[next]?.focus();
    onChange(options[next].value);
  }

  return (
    <div
      role={tabs ? "tablist" : "group"}
      aria-label={label}
      className={[styles.track, className].filter(Boolean).join(" ")}
    >
      {options.map((o, i) => {
        const selected = o.value === value;
        return (
          <button
            key={o.value}
            ref={(el) => {
              refs.current[i] = el;
            }}
            type="button"
            className={[styles.segment, selected && styles.selected].filter(Boolean).join(" ")}
            {...(tabs
              ? {
                  role: "tab",
                  id: `${idPrefix}-tab-${o.value}`,
                  "aria-selected": selected,
                  "aria-controls": `${idPrefix}-panel-${o.value}`,
                  tabIndex: selected ? 0 : -1,
                }
              : { "aria-pressed": selected })}
            onClick={() => onChange(o.value)}
            onKeyDown={(e) => onKeyDown(e, i)}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

import { Chip } from "@/components/Chip/Chip";
import styles from "./FilterChips.module.css";

export interface FilterOption {
  value: string;
  label: string;
  /** When set, the chip is a link (used where filters navigate). */
  href?: string;
}

interface FilterChipsProps {
  label: string;
  options: readonly FilterOption[];
  value: string;
  onChange?: (value: string) => void;
  /** Scroll sideways instead of wrapping on narrow screens. */
  scroll?: boolean;
  className?: string;
}

/** A row of Chips where exactly one is selected. */
export function FilterChips({ label, options, value, onChange, scroll = true, className }: FilterChipsProps) {
  return (
    <div
      role="group"
      aria-label={label}
      className={[styles.row, scroll && styles.scroll, className].filter(Boolean).join(" ")}
    >
      {options.map((o) =>
        o.href !== undefined ? (
          <Chip key={o.value} href={o.href} selected={o.value === value} scroll={false}>
            {o.label}
          </Chip>
        ) : (
          <Chip key={o.value} selected={o.value === value} onClick={() => onChange?.(o.value)}>
            {o.label}
          </Chip>
        ),
      )}
    </div>
  );
}

import Link from "next/link";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import styles from "./Chip.module.css";

interface BaseProps {
  selected?: boolean;
  children: ReactNode;
  className?: string;
}

type ChipButtonProps = BaseProps & Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> & { href?: undefined };
type ChipLinkProps = BaseProps & { href: string; replace?: boolean; scroll?: boolean };

/**
 * Pill. Selected chips are ink on bg. As a button it toggles with aria-pressed; with `href`
 * it navigates and marks the selected chip with aria-current.
 */
export function Chip(props: ChipButtonProps | ChipLinkProps) {
  const { selected = false, children, className } = props;
  const cls = [styles.chip, selected && styles.selected, className].filter(Boolean).join(" ");
  if (props.href !== undefined) {
    return (
      <Link
        href={props.href}
        replace={props.replace}
        scroll={props.scroll}
        className={cls}
        aria-current={selected ? "page" : undefined}
      >
        {children}
      </Link>
    );
  }
  const { selected: _s, children: _c, className: _cn, href: _h, type = "button", ...rest } = props;
  return (
    <button type={type} className={cls} aria-pressed={selected} {...rest}>
      {children}
    </button>
  );
}

import Link from "next/link";
import type { AnchorHTMLAttributes, ButtonHTMLAttributes, ReactNode } from "react";
import { Icon } from "@/components/Icon/Icon";
import styles from "./Button.module.css";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
export type ButtonSize = 48 | 44 | 36;

interface CommonProps {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Stretch to the container width. */
  block?: boolean;
  /** Shows a spinner, swaps the label for `loadingLabel` ("Posting…") and sets aria-busy. */
  loading?: boolean;
  loadingLabel?: string;
  icon?: ReactNode;
  children: ReactNode;
  className?: string;
}

type ButtonAsButton = CommonProps &
  Omit<ButtonHTMLAttributes<HTMLButtonElement>, keyof CommonProps> & { href?: undefined };
type ButtonAsLink = CommonProps &
  Omit<AnchorHTMLAttributes<HTMLAnchorElement>, keyof CommonProps> & { href: string; prefetch?: boolean };

export type ButtonProps = ButtonAsButton | ButtonAsLink;

export function buttonClass({
  variant = "primary",
  size = 48,
  block,
  className,
}: Pick<CommonProps, "variant" | "size" | "block" | "className">) {
  return [styles.button, styles[variant], styles[`size${size}`], block && styles.block, className]
    .filter(Boolean)
    .join(" ");
}

export function Button(props: ButtonProps) {
  const { variant, size, block, loading, loadingLabel, icon, children, className, ...rest } = props;
  const cls = buttonClass({ variant, size, block, className });
  const content = loading ? (
    <>
      <Icon name="spinner" size={18} strokeWidth={2.5} className={styles.spinner} />
      <span>{loadingLabel ?? children}</span>
    </>
  ) : (
    <>
      {icon}
      <span>{children}</span>
    </>
  );

  if (rest.href !== undefined) {
    const { href, ...anchor } = rest as ButtonAsLink;
    return (
      <Link href={href} className={cls} aria-busy={loading || undefined} {...anchor}>
        {content}
      </Link>
    );
  }

  const { type = "button", disabled, ...button } = rest as ButtonAsButton;
  return (
    <button
      type={type}
      className={cls}
      disabled={disabled}
      aria-busy={loading || undefined}
      aria-disabled={loading || undefined}
      data-loading={loading || undefined}
      {...button}
      onClick={loading ? (e) => e.preventDefault() : button.onClick}
    >
      {content}
    </button>
  );
}

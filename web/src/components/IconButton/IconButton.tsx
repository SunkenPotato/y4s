import type { ButtonHTMLAttributes, ReactNode } from "react";
import styles from "./IconButton.module.css";

interface IconButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "aria-label" | "children"> {
  /** Required: the button has no visible text. */
  "aria-label": string;
  children: ReactNode;
  variant?: "secondary" | "ghost";
}

/** 44×44 icon-only button. */
export function IconButton({ variant = "secondary", className, type = "button", ...rest }: IconButtonProps) {
  return <button type={type} className={[styles.button, styles[variant], className].filter(Boolean).join(" ")} {...rest} />;
}

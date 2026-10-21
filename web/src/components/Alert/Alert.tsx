import type { ReactNode } from "react";
import { Icon } from "@/components/Icon/Icon";
import styles from "./Alert.module.css";

export type AlertTone = "success" | "warning" | "error";

interface AlertProps {
  tone: AlertTone;
  /** Bold first line. */
  title?: ReactNode;
  children?: ReactNode;
  /** A trailing control, like a Retry button. */
  action?: ReactNode;
  id?: string;
  className?: string;
}

const ICON = { success: "check", warning: "warning", error: "error" } as const;

/** Soft fill, icon, bold first line. Errors use role="alert"; the rest are role="status". */
export function Alert({ tone, title, children, action, id, className }: AlertProps) {
  return (
    <div
      id={id}
      role={tone === "error" ? "alert" : "status"}
      className={[styles.alert, styles[tone], className].filter(Boolean).join(" ")}
    >
      <Icon name={ICON[tone]} size={20} strokeWidth={2.4} className={styles.icon} />
      <div className={styles.body}>
        {title ? <div className={styles.title}>{title}</div> : null}
        {children ? <div>{children}</div> : null}
      </div>
      {action ? <div className={styles.action}>{action}</div> : null}
    </div>
  );
}

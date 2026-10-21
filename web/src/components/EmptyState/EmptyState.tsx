import type { ReactNode } from "react";
import { Icon, type IconName } from "@/components/Icon/Icon";
import styles from "./EmptyState.module.css";

interface EmptyStateProps {
  icon?: IconName;
  title: ReactNode;
  children?: ReactNode;
  /** One primary action. */
  action?: ReactNode;
  headingLevel?: 1 | 2;
}

/** 64px surface icon tile, a Title 2 line, one sentence and one primary action. */
export function EmptyState({ icon = "pin", title, children, action, headingLevel = 2 }: EmptyStateProps) {
  const Heading = headingLevel === 1 ? "h1" : "h2";
  return (
    <div className={styles.empty}>
      <div className={styles.icon}>
        <Icon name={icon} size={28} />
      </div>
      <Heading className={styles.title}>{title}</Heading>
      {children ? <p className={styles.text}>{children}</p> : null}
      {action ? <div className={styles.action}>{action}</div> : null}
    </div>
  );
}

import { Icon } from "@/components/Icon/Icon";
import styles from "./StepList.module.css";

export interface Step {
  title: string;
  detail?: string;
  state: "done" | "current" | "todo";
}

/** A numbered sequence on surface rows: done (accent check), current (warn) and to-do (muted). */
export function StepList({ steps, label }: { steps: Step[]; label: string }) {
  return (
    <ol className={styles.list} aria-label={label}>
      {steps.map((s, i) => (
        <li key={s.title} className={styles.step} aria-current={s.state === "current" ? "step" : undefined}>
          <span className={[styles.mark, styles[s.state]].join(" ")}>
            {s.state === "done" ? <Icon name="check" size={16} strokeWidth={3} /> : i + 1}
          </span>
          <span>
            <span className={[styles.title, s.state === "todo" && styles.muted].filter(Boolean).join(" ")}>
              {s.title}
              {s.state === "done" ? <span className="visually-hidden"> (done)</span> : null}
            </span>
            {s.detail ? <span className={styles.detail}>{s.detail}</span> : null}
          </span>
        </li>
      ))}
    </ol>
  );
}

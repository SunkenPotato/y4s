import Link from "next/link";
import type { CSSProperties, ElementType, ReactNode } from "react";
import styles from "./Layout.module.css";

type Space = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8;
const space = (n: Space) => `var(--space-${n})`;

/** The content column: max-width var(--page-max), gutter padding, room for the TabBar on phones. */
export function Container({ children, narrow, className }: { children: ReactNode; narrow?: boolean; className?: string }) {
  return <div className={[styles.container, narrow && styles.narrow, className].filter(Boolean).join(" ")}>{children}</div>;
}

/** Main column plus aside: flex-wrap, the aside stacks below main on narrow screens. */
export function Columns({ children, gap = 7, className }: { children: ReactNode; gap?: Space; className?: string }) {
  return (
    <div className={[styles.columns, className].filter(Boolean).join(" ")} style={{ gap: space(gap) }}>
      {children}
    </div>
  );
}

export function MainColumn({ children, as: As = "div" }: { children: ReactNode; as?: ElementType }) {
  return <As className={styles.main}>{children}</As>;
}

export function Aside({ children, label }: { children: ReactNode; label?: string }) {
  return (
    <aside className={styles.aside} aria-label={label}>
      {children}
    </aside>
  );
}

/** Vertical stack with a token gap. */
export function Stack({
  children,
  gap = 4,
  as: As = "div",
  className,
  style,
}: {
  children: ReactNode;
  gap?: Space;
  as?: ElementType;
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <As className={[styles.stack, className].filter(Boolean).join(" ")} style={{ gap: space(gap), ...style }}>
      {children}
    </As>
  );
}

/** Horizontal wrapping row. */
export function Row({
  children,
  gap = 3,
  align = "center",
  justify,
  className,
}: {
  children: ReactNode;
  gap?: Space;
  align?: CSSProperties["alignItems"];
  justify?: CSSProperties["justifyContent"];
  className?: string;
}) {
  return (
    <div className={[styles.row, className].filter(Boolean).join(" ")} style={{ gap: space(gap), alignItems: align, justifyContent: justify }}>
      {children}
    </div>
  );
}

/** Responsive tile grid. */
export function TileGrid({ children, min = 320, label }: { children: ReactNode; min?: number; label?: string }) {
  return (
    <div className={styles.grid} style={{ "--grid-min": `${min}px` } as CSSProperties} aria-label={label} role={label ? "list" : undefined}>
      {children}
    </div>
  );
}

/** Page title (Display) with an optional muted line under it. */
export function PageHead({
  title,
  children,
  size = "display",
  className,
}: {
  title: ReactNode;
  children?: ReactNode;
  size?: "display" | "title";
  className?: string;
}) {
  return (
    <div className={[styles.head, className].filter(Boolean).join(" ")}>
      <h1 className={size === "display" ? styles.display : styles.title}>{title}</h1>
      {children ? <p className={styles.lede}>{children}</p> : null}
    </div>
  );
}

/** A section with a Title 2 heading. */
export function Section({
  title,
  children,
  action,
  id,
  className,
}: {
  title: ReactNode;
  children: ReactNode;
  action?: ReactNode;
  id?: string;
  className?: string;
}) {
  const headingId = id ? `${id}-title` : undefined;
  return (
    <section className={[styles.section, className].filter(Boolean).join(" ")} aria-labelledby={headingId} id={id}>
      <div className={styles.sectionHead}>
        <h2 id={headingId} className={styles.sectionTitle}>
          {title}
        </h2>
        {action}
      </div>
      {children}
    </section>
  );
}

/** "Near UCLA › Food" */
export function Crumbs({ items }: { items: Array<{ label: string; href?: string }> }) {
  return (
    <nav aria-label="Breadcrumb" className={styles.crumbs}>
      <ol>
        {items.map((item, i) => (
          <li key={`${item.label}-${i}`}>
            {item.href ? <Link href={item.href}>{item.label}</Link> : <span aria-current="page">{item.label}</span>}
            {i < items.length - 1 ? <span aria-hidden="true"> › </span> : null}
          </li>
        ))}
      </ol>
    </nav>
  );
}

/** Muted text line. */
export function Muted({ children, as: As = "p", small }: { children: ReactNode; as?: ElementType; small?: boolean }) {
  return <As className={[styles.muted, small && styles.small].filter(Boolean).join(" ")}>{children}</As>;
}

/** Text link in accent-ink, for inline actions like "Cancel" or "Not listed? Add your school". */
export function TextLink({
  href,
  children,
  onClick,
}: {
  href?: string;
  children: ReactNode;
  onClick?: () => void;
}) {
  if (href) {
    return (
      <Link href={href} className={styles.textLink}>
        {children}
      </Link>
    );
  }
  return (
    <button type="button" onClick={onClick} className={styles.textLink}>
      {children}
    </button>
  );
}

/** A quiet surface block shown while content loads. */
export function Placeholder({ height = 180, radius = "tile" }: { height?: number; radius?: "tile" | "hero" | "control" }) {
  return <div className={styles.placeholder} style={{ height, borderRadius: `var(--radius-${radius})` }} aria-hidden="true" />;
}

/** Screen-reader-only loading status for Suspense fallbacks. */
export function LoadingNote({ children = "Loading…" }: { children?: ReactNode }) {
  return (
    <p role="status" className="visually-hidden">
      {children}
    </p>
  );
}

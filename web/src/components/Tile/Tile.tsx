import type { ReactNode } from "react";
import styles from "./Tile.module.css";

interface TileProps {
  tone?: "surface" | "accent" | "warn";
  as?: "section" | "div" | "article" | "aside";
  /** 18px bold heading at the top of the tile. */
  title?: ReactNode;
  padding?: "default" | "tight";
  className?: string;
  children?: ReactNode;
  "aria-labelledby"?: string;
}

/** A soft filled tile for grouping, used in asides and info panels. */
export function Tile({ tone = "surface", as: As = "section", title, padding = "default", className, children, ...rest }: TileProps) {
  return (
    <As className={[styles.tile, styles[tone], padding === "tight" && styles.tight, className].filter(Boolean).join(" ")} {...rest}>
      {title ? <h2 className={styles.title}>{title}</h2> : null}
      {children}
    </As>
  );
}

import { initials } from "@/lib/format";
import styles from "./Avatar.module.css";

interface AvatarProps {
  name: string;
  size?: 30 | 36 | 44;
  /** "accent" is used only for the signed-in user's chip in the header. */
  tone?: "surface" | "accent";
}

/** Initials in a circle. Decorative: the name is always shown next to it. */
export function Avatar({ name, size = 36, tone = "surface" }: AvatarProps) {
  return (
    <span aria-hidden="true" className={[styles.avatar, styles[`s${size}`], styles[tone]].join(" ")}>
      {initials(name)}
    </span>
  );
}

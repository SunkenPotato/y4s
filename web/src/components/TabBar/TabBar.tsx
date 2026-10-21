"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { Icon, type IconName } from "@/components/Icon/Icon";
import styles from "./TabBar.module.css";

const TABS: Array<{ href: string; label: string; icon: IconName; match: (p: string) => boolean }> = [
  { href: "/", label: "Explore", icon: "pin", match: (p) => p === "/" || p.startsWith("/search") || (p.startsWith("/places") && !p.endsWith("/review")) },
  { href: "/deals", label: "Deals", icon: "tag", match: (p) => p.startsWith("/deals") },
  { href: "/review", label: "Review", icon: "pen", match: (p) => p.startsWith("/review") || p.endsWith("/review") },
  { href: "/me", label: "Profile", icon: "user", match: (p) => p.startsWith("/me") || p.startsWith("/login") || p.startsWith("/signup") },
];

/** Mobile-only bottom navigation (under 720px). The selected tab is accent. */
export function TabBar() {
  return <TabBarView pathname={usePathname() ?? "/"} />;
}

/** The same bar without reading the URL; the Suspense fallback while the pathname resolves. */
export function TabBarView({ pathname }: { pathname: string | null }) {
  return (
    <nav aria-label="Main" className={styles.bar}>
      {TABS.map((t) => {
        const active = pathname !== null && t.match(pathname);
        return (
          <Link key={t.href} href={t.href} className={[styles.tab, active && styles.active].filter(Boolean).join(" ")} aria-current={active ? "page" : undefined}>
            <Icon name={t.icon} size={24} />
            <span>{t.label}</span>
          </Link>
        );
      })}
    </nav>
  );
}

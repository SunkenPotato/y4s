import Link from "next/link";
import { Suspense } from "react";
import { Avatar } from "@/components/Avatar/Avatar";
import { buttonClass } from "@/components/Button/Button";
import type { Me } from "@/lib/api/types";
import styles from "./Header.module.css";
import { HeaderSearch, SearchForm } from "./HeaderSearch";

interface HeaderProps {
  /** null when signed out; undefined while the session is still loading. */
  me?: Me | null;
  /** "site": wordmark, search, nav. "minimal": wordmark only. "admin": wordmark, Admin, Back to site. */
  variant?: "site" | "minimal" | "admin";
  /** School name for the search placeholder. */
  schoolName?: string | null;
}

/** Wordmark, search field, then the nav. Wraps on narrow screens; the nav moves to the TabBar under 720px. */
export function Header({ me, variant = "site", schoolName }: HeaderProps) {
  if (variant === "admin") {
    return (
      <header className={styles.header}>
        <div className={styles.brand}>
          <Wordmark />
          <span className={styles.section}>Admin</span>
        </div>
        <nav aria-label="Admin" className={styles.adminNav}>
          <Link href="/" className={styles.navLink}>
            Back to site
          </Link>
        </nav>
      </header>
    );
  }

  if (variant === "minimal") {
    return (
      <header className={styles.header}>
        <Wordmark />
      </header>
    );
  }

  const school = schoolName ?? me?.school?.name;
  const label = school ? `Search places near ${school}` : "Search places";
  return (
    <header className={styles.header}>
      <Wordmark />
      <Suspense fallback={<SearchForm label={label} />}>
        <HeaderSearch label={label} />
      </Suspense>
      <nav aria-label="Main" className={styles.nav}>
        <Link href="/places/new" className={styles.navLink}>
          Add a place
        </Link>
        {me === undefined ? (
          <span className={styles.placeholder} aria-hidden="true" />
        ) : me ? (
          <>
            {me.role === "admin" ? (
              <Link href="/admin" className={styles.navLink}>
                Admin
              </Link>
            ) : null}
            <Link href="/me" className={styles.me} aria-label={`Your profile, ${me.first_name}`}>
              <Avatar name={me.display_name} size={30} tone="accent" />
              <span>{me.first_name}</span>
            </Link>
          </>
        ) : (
          <>
            <Link href="/login" className={styles.navLink}>
              Log in
            </Link>
            <Link href="/signup" className={buttonClass({ variant: "primary", size: 44 })}>
              Sign up
            </Link>
          </>
        )}
      </nav>
    </header>
  );
}

function Wordmark() {
  return (
    <Link href="/" className={styles.wordmark}>
      Corner
    </Link>
  );
}

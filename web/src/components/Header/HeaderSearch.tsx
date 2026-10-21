"use client";

import Form from "next/form";
import { useSearchParams } from "next/navigation";
import { SearchField } from "@/components/SearchField/SearchField";
import styles from "./Header.module.css";

interface SearchFormProps {
  label: string;
  defaultValue?: string;
}

/** The header search form. Submits a GET to /search with client-side navigation. */
export function SearchForm({ label, defaultValue = "" }: SearchFormProps) {
  return (
    <Form action="/search" className={styles.search} role="search">
      <SearchField key={defaultValue} name="q" label={label} placeholder={label} defaultValue={defaultValue} autoComplete="off" />
    </Form>
  );
}

/** SearchForm prefilled with the current ?q=. Needs a Suspense boundary (it reads the URL). */
export function HeaderSearch({ label }: { label: string }) {
  const q = useSearchParams().get("q") ?? "";
  return <SearchForm label={label} defaultValue={q} />;
}

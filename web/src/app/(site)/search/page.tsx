import type { Metadata } from "next";
import { SearchScreen } from "./SearchScreen";

export const metadata: Metadata = { title: "Search" };

export default function SearchPage({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) {
  return <SearchScreen searchParams={searchParams} mode="search" />;
}

import type { Metadata } from "next";
import { SearchScreen } from "../search/SearchScreen";

export const metadata: Metadata = { title: "Student deals" };

export default function DealsPage({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) {
  return <SearchScreen searchParams={searchParams} mode="deals" />;
}

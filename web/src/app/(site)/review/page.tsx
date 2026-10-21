import type { Metadata } from "next";
import { SearchScreen } from "../search/SearchScreen";

export const metadata: Metadata = { title: "Write a review" };

export default function ReviewPickerPage({ searchParams }: { searchParams: Promise<Record<string, string | string[] | undefined>> }) {
  return <SearchScreen searchParams={searchParams} mode="review" />;
}

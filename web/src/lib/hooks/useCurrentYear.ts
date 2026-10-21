"use client";

import { useSyncExternalStore } from "react";

const noop = () => () => {};

/** The viewer's current year, or undefined during SSR (dates then include the year). */
export function useCurrentYear(): number | undefined {
  return useSyncExternalStore(
    noop,
    () => new Date().getFullYear(),
    () => undefined,
  );
}

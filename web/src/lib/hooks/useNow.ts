"use client";

import { useSyncExternalStore } from "react";

/**
 * The current time, ticking every `intervalMs`. During SSR and hydration it returns
 * `serverNow` (the time the server rendered, or null), so markup matches; then it goes live.
 */
export function useNow(intervalMs = 1000, serverNow: number | null = null): number | null {
  return useSyncExternalStore(
    (onChange) => {
      const t = window.setInterval(onChange, intervalMs);
      return () => window.clearInterval(t);
    },
    () => Math.floor(Date.now() / intervalMs) * intervalMs,
    () => serverNow,
  );
}

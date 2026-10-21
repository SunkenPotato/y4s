import { createTransport } from "./core";
import { createApi } from "./endpoints";

/**
 * Browser API client. Calls /api/* on the web origin; next.config rewrites it to API_URL,
 * so the corner_session cookie is first-party and sent automatically.
 */
export const api = createApi(
  createTransport({
    baseUrl: "",
    // Only the mock layer reads this (it keeps its fake session in a readable cookie).
    cookie: () => (typeof document === "undefined" ? undefined : document.cookie),
  }),
);

export { ApiError, isApiError } from "./errors";
export type * from "./types";

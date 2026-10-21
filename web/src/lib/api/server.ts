import "server-only";
import { headers } from "next/headers";
import { createTransport } from "./core";
import { createApi, type Api } from "./endpoints";
import { isApiError } from "./errors";
import type { Me } from "./types";

const API_URL = process.env.API_URL ?? "http://localhost:4000";

/**
 * Server-side API client. Calls API_URL directly and forwards the incoming cookie header,
 * so the API sees the viewer's session. Must be awaited inside a Suspense boundary.
 */
export async function getServerApi(): Promise<Api> {
  const cookie = (await headers()).get("cookie") ?? undefined;
  return createApi(createTransport({ baseUrl: API_URL, cookie: () => cookie }));
}

/** The signed-in user, or null when there's no valid session. */
export async function getMe(api?: Api): Promise<Me | null> {
  const client = api ?? (await getServerApi());
  try {
    return await client.me();
  } catch (e) {
    if (isApiError(e) && (e.status === 401 || e.status === 403)) return null;
    throw e;
  }
}

export { ApiError, isApiError } from "./errors";
export type * from "./types";

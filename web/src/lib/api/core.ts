import { ApiError } from "./errors";

export type Method = "GET" | "POST" | "PATCH" | "PUT" | "DELETE";
export type QueryValue = string | number | boolean | null | undefined;

export interface RequestOptions {
  method?: Method;
  query?: Record<string, QueryValue>;
  body?: unknown;
  signal?: AbortSignal;
}

export type Transport = <T>(path: string, options?: RequestOptions) => Promise<T>;

export const MOCK = process.env.NEXT_PUBLIC_API_MOCK === "1";

export function buildQuery(query?: Record<string, QueryValue>): string {
  if (!query) return "";
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query)) {
    if (value === undefined || value === null || value === "") continue;
    params.set(key, String(value));
  }
  const s = params.toString();
  return s ? `?${s}` : "";
}

interface TransportConfig {
  /** "" in the browser (same-origin, rewritten by Next), API_URL on the server. */
  baseUrl: string;
  /** The cookie header to forward. Only the server sets this. */
  cookie?: () => string | undefined;
}

/**
 * Typed fetch wrapper. Throws ApiError on any non-2xx response or network failure.
 * Mutations always send Content-Type: application/json (the API's CSRF check needs it),
 * with `{}` when there's no body.
 */
export function createTransport({ baseUrl, cookie }: TransportConfig): Transport {
  return async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
    const method = options.method ?? "GET";

    if (MOCK) {
      const { mockRequest } = await import("./mocks");
      return mockRequest<T>(method, path, options.query ?? {}, options.body, cookie?.());
    }

    const headers: Record<string, string> = { Accept: "application/json" };
    let body: string | undefined;
    if (method !== "GET") {
      headers["Content-Type"] = "application/json";
      body = JSON.stringify(options.body ?? {});
    }
    const forwarded = cookie?.();
    if (forwarded) headers.Cookie = forwarded;

    let res: Response;
    try {
      res = await fetch(`${baseUrl}${path}${buildQuery(options.query)}`, {
        method,
        headers,
        body,
        credentials: "same-origin",
        cache: "no-store",
        signal: options.signal,
      });
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") throw e;
      throw ApiError.network();
    }

    if (!res.ok) throw await ApiError.fromResponse(res);
    if (res.status === 204) return undefined as T;
    const text = await res.text();
    return (text ? JSON.parse(text) : undefined) as T;
  };
}

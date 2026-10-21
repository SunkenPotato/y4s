import type { ApiErrorBody } from "./types";

type Extras = Omit<ApiErrorBody["error"], "code" | "message" | "fields">;

/** Thrown by every API call that gets a non-2xx response (or can't reach the API). */
export class ApiError extends Error {
  readonly code: string;
  readonly status: number;
  readonly fields: Record<string, string>;
  /** Extra keys on the error object, like attempts_left, retry_after, existing_id, review_id. */
  readonly extras: Extras;

  constructor(status: number, code: string, message: string, fields?: Record<string, string>, extras: Extras = {}) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.fields = fields ?? {};
    this.extras = extras;
  }

  get attemptsLeft(): number | undefined {
    return typeof this.extras.attempts_left === "number" ? this.extras.attempts_left : undefined;
  }

  get retryAfter(): number | undefined {
    return typeof this.extras.retry_after === "number" ? this.extras.retry_after : undefined;
  }

  static async fromResponse(res: Response): Promise<ApiError> {
    let body: Partial<ApiErrorBody> | null = null;
    try {
      body = (await res.json()) as Partial<ApiErrorBody>;
    } catch {
      body = null;
    }
    const err = body?.error;
    if (err && typeof err.code === "string") {
      const { code, message, fields, ...extras } = err;
      if (extras.retry_after === undefined) {
        const header = Number(res.headers.get("retry-after"));
        if (Number.isFinite(header) && header > 0) extras.retry_after = header;
      }
      return new ApiError(res.status, code, message || fallbackMessage(res.status), fields, extras);
    }
    return new ApiError(res.status, codeForStatus(res.status), fallbackMessage(res.status));
  }

  static network(): ApiError {
    return new ApiError(0, "network", "Couldn't reach Corner. Check your connection and try again.");
  }
}

export function isApiError(e: unknown): e is ApiError {
  return e instanceof ApiError;
}

function codeForStatus(status: number): string {
  switch (status) {
    case 401:
      return "unauthenticated";
    case 403:
      return "forbidden";
    case 404:
      return "not_found";
    case 422:
      return "validation";
    case 429:
      return "rate_limited";
    default:
      return "internal";
  }
}

function fallbackMessage(status: number): string {
  if (status === 404) return "We couldn't find that.";
  if (status === 401) return "Log in to continue.";
  if (status === 429) return "Too many tries. Wait a minute and try again.";
  return "Something went wrong on our side. Try again in a moment.";
}

import type { Transport } from "./core";
import type {
  AdminDomain,
  AdminPlace,
  AdminReport,
  ChooseSchoolInput,
  DomainStatus,
  Feed,
  Me,
  NewPlaceInput,
  Page,
  PlaceDetail,
  PlaceQuery,
  Place,
  ReportInput,
  ReportStatus,
  ResolveAction,
  Review,
  ReviewInput,
  ReviewSort,
  School,
  SchoolKind,
  SignupInput,
  Uuid,
  VoteResult,
} from "./types";

const enc = encodeURIComponent;

/** Every endpoint in docs/api.md, bound to a transport (browser, server or mock). */
export function createApi(t: Transport) {
  return {
    /* Auth */
    signup: (input: SignupInput) =>
      t<{ user: Me }>("/api/auth/signup", { method: "POST", body: input }).then((r) => r.user),
    chooseSchool: (input: ChooseSchoolInput) =>
      t<{ user: Me }>("/api/auth/school", { method: "POST", body: input }).then((r) => r.user),
    verify: (code: string) =>
      t<{ user: Me }>("/api/auth/verify", { method: "POST", body: { code } }).then((r) => r.user),
    resend: () => t<void>("/api/auth/resend", { method: "POST" }),
    login: (email: string, password: string) =>
      t<{ user: Me }>("/api/auth/login", { method: "POST", body: { email, password } }).then((r) => r.user),
    logout: () => t<void>("/api/auth/logout", { method: "POST" }),
    me: () => t<{ user: Me }>("/api/me").then((r) => r.user),
    updateMe: (input: { first_name?: string; last_name?: string }) =>
      t<{ user: Me }>("/api/me", { method: "PATCH", body: input }).then((r) => r.user),

    /* Schools */
    schools: (q: string, opts: { kind?: SchoolKind; limit?: number } = {}) =>
      t<{ items: School[] }>("/api/schools", { query: { q, kind: opts.kind, limit: opts.limit ?? 10 } }).then(
        (r) => r.items,
      ),

    /* Feed and places */
    feed: (schoolId?: Uuid) => t<Feed>("/api/feed", { query: { school_id: schoolId } }),
    places: (query: PlaceQuery = {}, signal?: AbortSignal) =>
      t<Page<Place>>("/api/places", { query: { ...query }, signal }),
    place: (id: Uuid) => t<{ place: PlaceDetail }>(`/api/places/${enc(id)}`).then((r) => r.place),
    createPlace: (input: NewPlaceInput) =>
      t<{ place: PlaceDetail }>("/api/places", { method: "POST", body: input }).then((r) => r.place),

    /* Reviews */
    reviews: (placeId: Uuid, opts: { sort?: ReviewSort; cursor?: string | null; limit?: number } = {}) =>
      t<Page<Review>>(`/api/places/${enc(placeId)}/reviews`, {
        query: { sort: opts.sort, cursor: opts.cursor, limit: opts.limit },
      }),
    createReview: (placeId: Uuid, input: ReviewInput) =>
      t<{ review: Review }>(`/api/places/${enc(placeId)}/reviews`, { method: "POST", body: input }).then(
        (r) => r.review,
      ),
    updateReview: (id: Uuid, input: Partial<ReviewInput>) =>
      t<{ review: Review }>(`/api/reviews/${enc(id)}`, { method: "PATCH", body: input }).then((r) => r.review),
    deleteReview: (id: Uuid) => t<void>(`/api/reviews/${enc(id)}`, { method: "DELETE" }),
    restoreReview: (id: Uuid) =>
      t<{ review: Review }>(`/api/reviews/${enc(id)}/restore`, { method: "POST" }).then((r) => r.review),
    vote: (id: Uuid) => t<VoteResult>(`/api/reviews/${enc(id)}/vote`, { method: "PUT" }),
    unvote: (id: Uuid) => t<VoteResult>(`/api/reviews/${enc(id)}/vote`, { method: "DELETE" }),

    /* Reports */
    report: (input: ReportInput) => t<{ id: Uuid }>("/api/reports", { method: "POST", body: input }),

    /* Admin */
    adminDomains: (status: DomainStatus) =>
      t<{ items: AdminDomain[] }>("/api/admin/domains", { query: { status } }).then((r) => r.items),
    approveDomain: (domain: string) =>
      t<AdminDomain>(`/api/admin/domains/${enc(domain)}/approve`, { method: "POST" }),
    rejectDomain: (domain: string, reason: string) =>
      t<AdminDomain>(`/api/admin/domains/${enc(domain)}/reject`, { method: "POST", body: { reason } }),
    resetDomain: (domain: string) => t<AdminDomain>(`/api/admin/domains/${enc(domain)}/reset`, { method: "POST" }),
    adminPlaces: (cursor?: string | null) =>
      t<Page<AdminPlace>>("/api/admin/places", { query: { status: "pending", cursor } }),
    approvePlace: (id: Uuid) =>
      t<{ place: PlaceDetail }>(`/api/admin/places/${enc(id)}/approve`, { method: "POST" }).then((r) => r.place),
    hidePlace: (id: Uuid) =>
      t<{ place: PlaceDetail }>(`/api/admin/places/${enc(id)}/hide`, { method: "POST" }).then((r) => r.place),
    adminReports: (status: ReportStatus, cursor?: string | null) =>
      t<Page<AdminReport>>("/api/admin/reports", { query: { status, cursor } }),
    resolveReport: (id: Uuid, action: ResolveAction) =>
      t<unknown>(`/api/admin/reports/${enc(id)}/resolve`, { method: "POST", body: { action } }),
  };
}

export type Api = ReturnType<typeof createApi>;

/**
 * Mirrors the shared objects in docs/api.md (v1). Keep these in lockstep with the contract.
 */

export type Uuid = string;
/** RFC 3339 UTC timestamp. */
export type Timestamp = string;

export type SchoolKind = "university" | "high_school";

export interface SchoolSummary {
  id: Uuid;
  name: string;
  kind: SchoolKind;
  city: string;
  state: string;
}

export interface School extends SchoolSummary {
  /** Null for schools students added themselves until someone places them. */
  lat: number | null;
  lng: number | null;
}

export type Role = "user" | "admin";
export type UserStatus = "pending_email" | "pending_school" | "verified" | "rejected" | "suspended";

export interface Me {
  id: Uuid;
  email: string;
  first_name: string;
  last_name: string;
  display_name: string;
  role: Role;
  status: UserStatus;
  school: SchoolSummary | null;
  needs_school: boolean;
  created_at: Timestamp;
}

export interface Author {
  display_name: string;
  school_name: string;
  school_kind: SchoolKind;
}

export const CATEGORIES = [
  "food",
  "coffee",
  "study",
  "gym",
  "services",
  "nightlife",
  "groceries",
  "other",
] as const;
export type Category = (typeof CATEGORIES)[number];

export type PlaceKind = "business" | "facility";
export type PriceLevel = 1 | 2 | 3 | 4;
export type PlaceStatus = "active" | "pending" | "hidden";

export interface Quote {
  text: string;
  author: Author;
  review_id: Uuid;
}

export interface Place {
  id: Uuid;
  kind: PlaceKind;
  name: string;
  category: Category;
  subcategory: string | null;
  price_level: PriceLevel | null;
  address: string;
  lat: number;
  lng: number;
  school_id: Uuid | null;
  student_discount: string | null;
  rating_avg: number | null;
  review_count: number;
  distance_mi: number | null;
  top_quote: Quote | null;
  status: PlaceStatus;
}

export interface HoursRow {
  days: string;
  time: string;
}

export type RatingDistribution = Record<"1" | "2" | "3" | "4" | "5", number>;

export interface PlaceDetail extends Place {
  hours: HoursRow[] | null;
  rating_distribution: RatingDistribution;
  school: SchoolSummary | null;
  viewer_review_id: Uuid | null;
}

export type Rating = 1 | 2 | 3 | 4 | 5;

export interface Review {
  id: Uuid;
  place_id: Uuid;
  rating: Rating;
  body: string;
  visited_month: string | null;
  used_deal: boolean;
  helpful_count: number;
  viewer_voted: boolean;
  is_mine: boolean;
  author: Author;
  created_at: Timestamp;
  updated_at: Timestamp;
}

export interface Page<T> {
  items: T[];
  next_cursor: string | null;
}

export interface Feed {
  school: School;
  featured: { place: Place; quote: Quote } | null;
  popular: Place[];
  deals: Place[];
}

export type PlaceSort = "reviews" | "rating" | "distance" | "new";
export type ReviewSort = "helpful" | "new" | "lowest";

export interface PlaceQuery {
  q?: string;
  category?: Category;
  kind?: PlaceKind;
  school_id?: Uuid;
  near?: string;
  radius_mi?: number;
  deals?: boolean;
  sort?: PlaceSort;
  cursor?: string;
  limit?: number;
}

export interface SignupInput {
  email: string;
  password: string;
  first_name: string;
  last_name: string;
  birth_year: number;
}

export interface NewSchoolInput {
  name: string;
  kind: SchoolKind;
  city: string;
  state: string;
}

export type ChooseSchoolInput = { school_id: Uuid } | { new_school: NewSchoolInput };

export interface NewPlaceInput {
  kind: PlaceKind;
  name: string;
  category: Category;
  subcategory?: string | null;
  address: string;
  lat?: number;
  lng?: number;
  school_id?: Uuid | null;
  price_level?: PriceLevel | null;
  student_discount?: string | null;
  hours?: HoursRow[] | null;
}

export interface ReviewInput {
  rating: Rating;
  body: string;
  visited_month: string | null;
  used_deal: boolean;
}

export interface VoteResult {
  helpful_count: number;
  viewer_voted: boolean;
}

export type ReportTarget = "review" | "place";
export type ReportReason = "spam" | "offensive" | "not_a_student" | "wrong_info" | "other";

export interface ReportInput {
  target_type: ReportTarget;
  target_id: Uuid;
  reason: ReportReason;
  details?: string;
}

/* Admin */

export type DomainStatus = "pending" | "approved" | "rejected";

export interface AdminDomain {
  domain: string;
  status: DomainStatus;
  school: SchoolSummary;
  waiting_count: number;
  requested_at: Timestamp;
  decided_at: Timestamp | null;
  flags: string[];
}

export interface AdminPlace extends PlaceDetail {
  created_by: Author;
}

export type ReportStatus = "open" | "resolved";

export interface AdminReport {
  id: Uuid;
  target_type: ReportTarget;
  target_id: Uuid;
  target_preview: string;
  reason: ReportReason;
  details: string | null;
  reporter: Author;
  report_count: number;
  status: ReportStatus;
  created_at: Timestamp;
}

export type ResolveAction = "dismiss" | "remove_content";

/* Error envelope */

export interface ApiErrorBody {
  error: {
    code: string;
    message: string;
    fields?: Record<string, string>;
    /* Extra keys the contract mentions on specific errors. */
    attempts_left?: number;
    retry_after?: number;
    existing_id?: Uuid;
    review_id?: Uuid;
    [key: string]: unknown;
  };
}

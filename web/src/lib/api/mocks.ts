/**
 * Fixture API used when NEXT_PUBLIC_API_MOCK=1. Same fictional data as the design mockups.
 *
 * The fake session lives in a readable cookie (corner_mock_me) holding the Me object, so the
 * server render and the browser agree on who is signed in. Everything else is in-memory per
 * runtime: writes made in the browser are not seen by the server render, which is fine for
 * developing UI.
 */
import type { Method, QueryValue } from "./core";
import { ApiError } from "./errors";
import type {
  AdminDomain,
  AdminPlace,
  AdminReport,
  Author,
  Category,
  Feed,
  Me,
  Page,
  Place,
  PlaceDetail,
  Review,
  School,
  SchoolSummary,
} from "./types";

export const MOCK_COOKIE = "corner_mock_me";

/* Schools */

const UCLA: School = {
  id: "8d6c1f8e-0000-4000-8000-000000000001",
  name: "UCLA",
  kind: "university",
  city: "Los Angeles",
  state: "CA",
  lat: 34.0689,
  lng: -118.4452,
};

const SCHOOLS: School[] = [
  UCLA,
  { id: "8d6c1f8e-0000-4000-8000-000000000002", name: "University High School", kind: "high_school", city: "Los Angeles", state: "CA", lat: 34.0453, lng: -118.4581 },
  { id: "8d6c1f8e-0000-4000-8000-000000000003", name: "University High School", kind: "high_school", city: "Irvine", state: "CA", lat: 33.6461, lng: -117.8268 },
  { id: "8d6c1f8e-0000-4000-8000-000000000004", name: "Santa Monica College", kind: "university", city: "Santa Monica", state: "CA", lat: 34.0167, lng: -118.4703 },
  { id: "8d6c1f8e-0000-4000-8000-000000000005", name: "Beverly Hills High School", kind: "high_school", city: "Beverly Hills", state: "CA", lat: 34.0615, lng: -118.4058 },
  { id: "8d6c1f8e-0000-4000-8000-000000000006", name: "Westwood Charter", kind: "high_school", city: "Los Angeles", state: "CA", lat: 34.0556, lng: -118.4406 },
  { id: "8d6c1f8e-0000-4000-8000-000000000007", name: "Harvard-Westlake School", kind: "high_school", city: "Los Angeles", state: "CA", lat: 34.1394, lng: -118.4122 },
  { id: "8d6c1f8e-0000-4000-8000-000000000008", name: "Palisades Charter High", kind: "high_school", city: "Pacific Palisades", state: "CA", lat: 34.0469, lng: -118.5262 },
];

const summary = (s: School): SchoolSummary => ({ id: s.id, name: s.name, kind: s.kind, city: s.city, state: s.state });
const schoolByName = (name: string) => SCHOOLS.find((s) => s.name === name) ?? UCLA;

const author = (display_name: string, school = "UCLA"): Author => ({
  display_name,
  school_name: school,
  school_kind: schoolByName(school).kind,
});

/* Places */

const pid = (n: number) => `5b1e0c2a-0000-4000-8000-${String(n).padStart(12, "0")}`;
const rid = (n: number) => `9a7f3d10-0000-4000-8000-${String(n).padStart(12, "0")}`;

interface Seed {
  n: number;
  kind: Place["kind"];
  name: string;
  category: Category;
  subcategory: string | null;
  price: Place["price_level"];
  address: string;
  lat: number;
  lng: number;
  deal: string | null;
  rating: number;
  count: number;
  distance: number | null;
  quote: string;
  by: string;
  bySchool?: string;
}

const SEEDS: Seed[] = [
  { n: 1, kind: "business", name: "Gayley Noodle Bar", category: "food", subcategory: "Ramen", price: 1, address: "1062 Gayley Ave, Los Angeles, CA 90024", lat: 34.0632, lng: -118.4469, deal: "15% off Mon–Thu", rating: 4.6, count: 212, distance: 0.3, quote: "The spicy miso is worth the line. Go before six or you will wait half an hour.", by: "Maya R." },
  { n: 2, kind: "facility", name: "Main Library, third floor", category: "study", subcategory: "Study spot", price: null, address: "Powell Library, Los Angeles, CA 90095", lat: 34.0716, lng: -118.4422, deal: null, rating: 4.4, count: 389, distance: null, quote: "Quiet floor actually means quiet. Outlets are only on the window side.", by: "Daniel K." },
  { n: 3, kind: "business", name: "Strathmore Tacos", category: "food", subcategory: "Mexican", price: 1, address: "555 Strathmore Dr, Los Angeles, CA 90024", lat: 34.0668, lng: -118.4508, deal: "$2 tacos Tuesdays", rating: 4.7, count: 301, distance: 0.6, quote: "Al pastor is the move. Paying cash is faster at lunch.", by: "Priya S.", bySchool: "University High School" },
  { n: 4, kind: "business", name: "Lindbrook Coffee", category: "coffee", subcategory: "Coffee", price: 1, address: "10900 Lindbrook Dr, Los Angeles, CA 90024", lat: 34.0603, lng: -118.4436, deal: "Free drip refill", rating: 4.2, count: 97, distance: 0.5, quote: "Good wifi, small tables. It gets loud after two.", by: "Jonah L.", bySchool: "Santa Monica College" },
  { n: 5, kind: "facility", name: "Wooden Rec Center", category: "gym", subcategory: "Gym", price: null, address: "221 Westwood Plaza, Los Angeles, CA 90095", lat: 34.0711, lng: -118.4459, deal: null, rating: 3.9, count: 156, distance: null, quote: "Four to seven is rough. Mornings are nearly empty.", by: "Alex T." },
  { n: 6, kind: "business", name: "Westwood Copy & Print", category: "services", subcategory: "Printing", price: 1, address: "1001 Westwood Blvd, Los Angeles, CA 90024", lat: 34.0617, lng: -118.4468, deal: "10% off binding", rating: 3.8, count: 41, distance: 0.4, quote: "Fast turnaround, but check the margins before they print.", by: "Sam W." },
];

const GAYLEY_HOURS = [
  { days: "Mon – Thu", time: "11 am – 10 pm" },
  { days: "Fri", time: "11 am – 11 pm" },
  { days: "Sat", time: "12 pm – 11 pm" },
  { days: "Sun", time: "12 pm – 9 pm" },
];

function seedToDetail(s: Seed): PlaceDetail {
  const total = s.count;
  const five = Math.round(total * 0.6);
  const four = Math.round(total * 0.25);
  const three = Math.round(total * 0.09);
  const two = Math.round(total * 0.04);
  return {
    id: pid(s.n),
    kind: s.kind,
    name: s.name,
    category: s.category,
    subcategory: s.subcategory,
    price_level: s.price,
    address: s.address,
    lat: s.lat,
    lng: s.lng,
    school_id: s.kind === "facility" ? UCLA.id : null,
    student_discount: s.deal,
    rating_avg: s.rating,
    review_count: s.count,
    distance_mi: s.distance,
    top_quote: { text: s.quote, author: author(s.by, s.bySchool), review_id: rid(s.n * 100) },
    status: "active",
    hours: s.n === 1 ? GAYLEY_HOURS : s.kind === "facility" ? [{ days: "Mon – Fri", time: "7 am – 11 pm" }, { days: "Sat – Sun", time: "9 am – 6 pm" }] : [{ days: "Daily", time: "8 am – 8 pm" }],
    rating_distribution:
      s.n === 1
        ? { "1": 5, "2": 8, "3": 19, "4": 52, "5": 128 }
        : { "1": Math.max(0, total - five - four - three - two), "2": two, "3": three, "4": four, "5": five },
    school: s.kind === "facility" ? summary(UCLA) : null,
    viewer_review_id: null,
  };
}

const places: PlaceDetail[] = SEEDS.map(seedToDetail);

const toPlace = (p: PlaceDetail): Place => {
  const { hours: _h, rating_distribution: _d, school: _s, viewer_review_id: _v, ...rest } = p;
  return rest;
};

/* Reviews */

const GAYLEY_REVIEWS: Array<[string, string, number, number, string, string]> = [
  ["Maya R.", "UCLA", 5, 24, "2026-10-14T19:20:00Z", "Spicy miso with the extra egg is the order. Portions are big enough that I take half home. The line is out the door by 6:30 on weeknights, so come early or order ahead."],
  ["Daniel K.", "UCLA", 4, 17, "2026-10-09T18:02:00Z", "Broth is great, noodles a bit soft for me. The student discount only works Monday to Thursday, and you have to show it before you pay, not after."],
  ["Priya S.", "University High School", 5, 9, "2026-10-02T21:40:00Z", "Came here with friends after a game. Cheap, fast, and they did not rush us out even though we stayed an hour."],
  ["Jonah L.", "Santa Monica College", 3, 6, "2026-09-27T20:15:00Z", "Good food, but it is small and loud. Not a place to sit and study. Fine for a quick bowl before class though."],
];

let reviews: Review[] = [];
SEEDS.forEach((s) => {
  const rows =
    s.n === 1
      ? GAYLEY_REVIEWS
      : ([[s.by, s.bySchool ?? "UCLA", Math.round(s.rating), 12, "2026-10-12T17:00:00Z", `${s.quote} Worth a visit if you're nearby, and the price is fair for students on a budget.`]] as typeof GAYLEY_REVIEWS);
  rows.forEach(([name, school, rating, helpful, at, body], i) => {
    reviews.push({
      id: rid(s.n * 100 + i),
      place_id: pid(s.n),
      rating: rating as Review["rating"],
      body,
      visited_month: at.slice(0, 7),
      used_deal: i === 1,
      helpful_count: helpful,
      viewer_voted: false,
      is_mine: false,
      author: author(name, school),
      created_at: at,
      updated_at: at,
    });
  });
});
const deletedReviews = new Map<string, Review>();

/* Admin */

let domains: AdminDomain[] = [
  { domain: "students.lausd.net", status: "pending", school: summary(SCHOOLS[1]), waiting_count: 4, requested_at: "2026-10-19T16:00:00Z", decided_at: null, flags: [] },
  { domain: "stu.bhusd.org", status: "pending", school: summary(SCHOOLS[4]), waiting_count: 1, requested_at: "2026-10-20T16:00:00Z", decided_at: null, flags: [] },
  { domain: "gmail.com", status: "pending", school: summary(SCHOOLS[5]), waiting_count: 1, requested_at: "2026-10-21T09:00:00Z", decided_at: null, flags: ["personal_provider"] },
  { domain: "smc.edu", status: "approved", school: summary(SCHOOLS[3]), waiting_count: 0, requested_at: "2026-10-10T16:00:00Z", decided_at: "2026-10-18T16:00:00Z", flags: [] },
  { domain: "mail.harvard-westlake.org", status: "approved", school: summary(SCHOOLS[6]), waiting_count: 0, requested_at: "2026-10-11T16:00:00Z", decided_at: "2026-10-17T16:00:00Z", flags: [] },
  { domain: "outlook.com", status: "rejected", school: summary(SCHOOLS[7]), waiting_count: 1, requested_at: "2026-10-12T16:00:00Z", decided_at: "2026-10-16T16:00:00Z", flags: ["personal_provider"] },
];

const pendingPlaces: AdminPlace[] = [
  { ...seedToDetail({ n: 7, kind: "business", name: "Sawtelle Boba Lab", category: "coffee", subcategory: "Boba", price: 1, address: "2000 Sawtelle Blvd, Los Angeles, CA 90025", lat: 34.0412, lng: -118.4432, deal: "Buy 5, get 1 free", rating: 0, count: 0, distance: 1.4, quote: "", by: "Maya R." }), rating_avg: null, top_quote: null, status: "pending", created_by: author("Maya R.") },
  { ...seedToDetail({ n: 8, kind: "facility", name: "Court of Sciences, east benches", category: "study", subcategory: "Outdoor seating", price: null, address: "Court of Sciences, Los Angeles, CA 90095", lat: 34.0689, lng: -118.4421, deal: null, rating: 0, count: 0, distance: null, quote: "", by: "Daniel K." }), rating_avg: null, top_quote: null, status: "pending", created_by: author("Daniel K.") },
];

let reports: AdminReport[] = [
  { id: "c0ffee00-0000-4000-8000-000000000001", target_type: "review", target_id: rid(103), target_preview: "Good food, but it is small and loud. Not a place to sit and study.", reason: "not_a_student", details: "This account posts about places all over the country.", reporter: author("Alex T."), report_count: 3, status: "open", created_at: "2026-10-20T12:00:00Z" },
  { id: "c0ffee00-0000-4000-8000-000000000002", target_type: "place", target_id: pid(6), target_preview: "Westwood Copy & Print", reason: "wrong_info", details: "They moved to 1050 Westwood Blvd in September.", reporter: author("Sam W."), report_count: 1, status: "open", created_at: "2026-10-21T08:00:00Z" },
];

/* Session */

const PERSONAL = /^(gmail\.com|googlemail\.com|yahoo\.[a-z.]+|outlook\.com|hotmail\.[a-z.]+|live\.com|icloud\.com|me\.com|aol\.com|proton\.me|protonmail\.com|gmx\.[a-z.]+|mail\.com|yandex\.[a-z.]+|zoho\.com)$/;

function readSession(cookie?: string): Me | null {
  if (!cookie) return null;
  const raw = cookie
    .split(/;\s*/)
    .find((c) => c.startsWith(`${MOCK_COOKIE}=`))
    ?.slice(MOCK_COOKIE.length + 1);
  if (!raw) return null;
  try {
    return JSON.parse(decodeURIComponent(raw)) as Me;
  } catch {
    return null;
  }
}

function writeSession(me: Me | null) {
  if (typeof document === "undefined") return;
  document.cookie = me
    ? `${MOCK_COOKIE}=${encodeURIComponent(JSON.stringify(me))}; Path=/; Max-Age=2592000; SameSite=Lax`
    : `${MOCK_COOKIE}=; Path=/; Max-Age=0; SameSite=Lax`;
}

function makeMe(email: string, first: string, last: string, patch: Partial<Me> = {}): Me {
  return {
    id: "7e57ab1e-0000-4000-8000-000000000001",
    email,
    first_name: first,
    last_name: last,
    display_name: `${first} ${last.charAt(0)}.`,
    role: "user",
    status: "verified",
    school: summary(UCLA),
    needs_school: false,
    created_at: "2026-09-01T12:00:00Z",
    ...patch,
  };
}

let codeAttempts = 5;

/* Helpers */

const fail = (status: number, code: string, message: string, fields?: Record<string, string>, extras = {}) => {
  throw new ApiError(status, code, message, fields, extras);
};

function requireMe(cookie?: string, verified = false): Me {
  const me = readSession(cookie);
  if (!me) return fail(401, "unauthenticated", "Log in to continue.");
  if (verified && me.status !== "verified") return fail(403, "not_verified", "Verify your school email to do this.");
  return me;
}

function requireAdmin(cookie?: string): Me {
  const me = requireMe(cookie, true);
  if (me.role !== "admin") return fail(403, "forbidden", "Only admins can do this.");
  return me;
}

function paginate<T>(items: T[], query: Record<string, QueryValue>): Page<T> {
  const limit = Math.min(Number(query.limit ?? 20) || 20, 50);
  const start = Number(query.cursor ?? 0) || 0;
  const slice = items.slice(start, start + limit);
  return { items: slice, next_cursor: start + limit < items.length ? String(start + limit) : null };
}

function viewReview(r: Review, me: Me | null): Review {
  return { ...r, is_mine: !!me && r.author.display_name === me.display_name && r.id.endsWith("mine") };
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/* Router */

export async function mockRequest<T>(
  method: Method,
  path: string,
  query: Record<string, QueryValue>,
  body: unknown,
  cookie?: string,
): Promise<T> {
  if (typeof window !== "undefined") await sleep(250);
  return route(method, path, query, (body ?? {}) as Record<string, unknown>, cookie) as T;
}

function route(method: Method, path: string, query: Record<string, QueryValue>, body: Record<string, unknown>, cookie?: string): unknown {
  const me = readSession(cookie);
  let m: RegExpMatchArray | null;

  /* Auth */
  if (method === "POST" && path === "/api/auth/signup") {
    const email = String(body.email ?? "").trim().toLowerCase();
    const fields: Record<string, string> = {};
    if (!/^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email)) fields.email = "Enter your school email, like name@ucla.edu.";
    if (String(body.password ?? "").length < 10) fields.password = "Use at least 10 characters.";
    if (!String(body.first_name ?? "").trim()) fields.first_name = "Enter your first name.";
    if (!String(body.last_name ?? "").trim()) fields.last_name = "Enter your last name.";
    const year = Number(body.birth_year);
    if (!year || year < 1900) fields.birth_year = "Enter the year you were born.";
    if (Object.keys(fields).length) fail(422, "validation", "Check the highlighted fields.", fields);
    const domain = email.split("@")[1];
    if (PERSONAL.test(domain)) fail(422, "not_school_email", "Use the email your school gave you, like name@ucla.edu.", { email: "Use the email your school gave you, like name@ucla.edu." });
    if (new Date().getFullYear() - year < 13) fail(422, "too_young", "Corner is for students 13 and older.", { birth_year: "Corner is for students 13 and older." });
    if (email === "taken@ucla.edu") fail(409, "email_taken", "There's already an account with this email. Log in instead.");
    const known = domain === "g.ucla.edu" || domain === "ucla.edu";
    const user = makeMe(email, String(body.first_name).trim(), String(body.last_name).trim(), {
      status: "pending_email",
      school: known ? summary(UCLA) : null,
      needs_school: !known,
    });
    codeAttempts = 5;
    writeSession(user);
    return { user };
  }

  if (method === "POST" && path === "/api/auth/school") {
    const current = requireMe(cookie);
    if (!current.needs_school) fail(403, "forbidden", "Your school is already set.");
    let school: SchoolSummary;
    if (typeof body.school_id === "string") {
      const found = SCHOOLS.find((s) => s.id === body.school_id);
      if (!found) return fail(404, "not_found", "We couldn't find that school.");
      school = summary(found);
    } else {
      const ns = (body.new_school ?? {}) as Record<string, string>;
      const fields: Record<string, string> = {};
      if (!ns.name?.trim()) fields["new_school.name"] = "Enter your school's name.";
      if (!ns.city?.trim()) fields["new_school.city"] = "Enter the city.";
      if (!ns.state?.trim()) fields["new_school.state"] = "Enter the state.";
      if (Object.keys(fields).length) fail(422, "validation", "Check the highlighted fields.", fields);
      school = { id: "8d6c1f8e-0000-4000-8000-0000000000ff", name: ns.name, kind: ns.kind === "university" ? "university" : "high_school", city: ns.city, state: ns.state };
    }
    const user: Me = { ...current, school, needs_school: false };
    writeSession(user);
    return { user };
  }

  if (method === "POST" && path === "/api/auth/verify") {
    const current = requireMe(cookie);
    if (current.status === "verified") fail(409, "already_verified", "Your email is already confirmed.");
    const code = String(body.code ?? "");
    if (codeAttempts <= 0) fail(422, "too_many_attempts", "Too many tries for this code. Send a new code and try again.");
    if (code === "000000" || !/^\d{6}$/.test(code)) {
      codeAttempts -= 1;
      if (codeAttempts <= 0) fail(422, "too_many_attempts", "Too many tries for this code. Send a new code and try again.");
      fail(422, "invalid_code", "That code didn't work.", undefined, { attempts_left: codeAttempts });
    }
    if (code === "999999") fail(422, "code_expired", "That code expired. Send a new code and try again.");
    const domain = current.email.split("@")[1];
    const edu = domain.endsWith(".edu");
    const status: Me["status"] = current.needs_school || !edu ? "pending_school" : "verified";
    const user: Me = { ...current, status, role: current.email === "admin@ucla.edu" ? "admin" : "user" };
    writeSession(user);
    return { user };
  }

  if (method === "POST" && path === "/api/auth/resend") {
    requireMe(cookie);
    codeAttempts = 5;
    return undefined;
  }

  if (method === "POST" && path === "/api/auth/login") {
    const email = String(body.email ?? "").trim().toLowerCase();
    const password = String(body.password ?? "");
    if (!email || password.length < 1 || password === "wrongpassword") fail(401, "invalid_credentials", "That email and password don't match. Check both and try again.");
    if (email.startsWith("suspended")) fail(403, "suspended", "This account is suspended. Email us if you think that's a mistake.");
    let user: Me;
    if (email === "admin@ucla.edu") user = makeMe(email, "Ari", "Admin", { role: "admin" });
    else if (email.includes("lausd")) user = makeMe(email, "Priya", "Shah", { status: "pending_school", school: summary(SCHOOLS[1]) });
    else if (email.includes("pending")) user = makeMe(email, "Maya", "Rivera", { status: "pending_email" });
    else user = makeMe(email.includes("@") ? email : "maya.r@g.ucla.edu", "Maya", "Rivera");
    writeSession(user);
    return { user };
  }

  if (method === "POST" && path === "/api/auth/logout") {
    writeSession(null);
    return undefined;
  }

  if (path === "/api/me") {
    const current = requireMe(cookie);
    if (method === "GET") return { user: current };
    if (method === "PATCH") {
      const fields: Record<string, string> = {};
      const first = body.first_name === undefined ? current.first_name : String(body.first_name).trim();
      const last = body.last_name === undefined ? current.last_name : String(body.last_name).trim();
      if (!first || first.length > 40) fields.first_name = "Use 1 to 40 characters.";
      if (!last || last.length > 40) fields.last_name = "Use 1 to 40 characters.";
      if (Object.keys(fields).length) fail(422, "validation", "Check the highlighted fields.", fields);
      const user: Me = { ...current, first_name: first, last_name: last, display_name: `${first} ${last.charAt(0)}.` };
      writeSession(user);
      return { user };
    }
  }

  /* Schools */
  if (method === "GET" && path === "/api/schools") {
    const q = String(query.q ?? "").trim().toLowerCase();
    const items = SCHOOLS.filter((s) => !q || s.name.toLowerCase().split(/\s+/).some((w) => w.startsWith(q)) || s.name.toLowerCase().startsWith(q))
      .filter((s) => !query.kind || s.kind === query.kind)
      .slice(0, Number(query.limit ?? 10));
    return { items };
  }

  /* Feed and places */
  if (method === "GET" && path === "/api/feed") {
    const active = places.filter((p) => p.status === "active").map(toPlace);
    const featuredPlace = active[0];
    const feed: Feed = {
      school: UCLA,
      featured: featuredPlace?.top_quote ? { place: featuredPlace, quote: featuredPlace.top_quote } : null,
      popular: active.slice(1, 9),
      deals: active.filter((p) => p.student_discount).slice(0, 6),
    };
    return feed;
  }

  if (method === "GET" && path === "/api/places") {
    const q = String(query.q ?? "").trim().toLowerCase();
    let items = places.filter((p) => p.status === "active").map(toPlace);
    if (q) items = items.filter((p) => [p.name, p.subcategory ?? "", p.address].some((t) => t.toLowerCase().includes(q)));
    if (query.category) items = items.filter((p) => p.category === query.category);
    if (query.kind) items = items.filter((p) => p.kind === query.kind);
    if (query.deals === true || query.deals === "true") items = items.filter((p) => p.student_discount);
    if (query.sort === "rating") items.sort((a, b) => (b.rating_avg ?? 0) - (a.rating_avg ?? 0));
    else items.sort((a, b) => b.review_count - a.review_count);
    return paginate(items, query);
  }

  if (method === "POST" && path === "/api/places") {
    requireMe(cookie, true);
    const fields: Record<string, string> = {};
    if (!String(body.name ?? "").trim()) fields.name = "Enter the place's name.";
    if (!String(body.address ?? "").trim()) fields.address = "Enter the street address.";
    if (Object.keys(fields).length) fail(422, "validation", "Check the highlighted fields.", fields);
    if (String(body.address).toLowerCase().includes("nowhere")) fail(422, "geocode_failed", "We couldn't find this address. Check the street and city.", { address: "We couldn't find this address. Check the street and city." });
    const existing = places.find((p) => p.name.toLowerCase() === String(body.name).trim().toLowerCase());
    if (existing) fail(409, "duplicate_place", `${existing.name} is already on Corner.`, undefined, { existing_id: existing.id });
    const place: PlaceDetail = {
      ...seedToDetail({ n: 900 + places.length, kind: body.kind === "facility" ? "facility" : "business", name: String(body.name).trim(), category: (body.category as Category) ?? "other", subcategory: (body.subcategory as string) || null, price: (body.price_level as Place["price_level"]) ?? null, address: String(body.address).trim(), lat: 34.0689, lng: -118.4452, deal: (body.student_discount as string) || null, rating: 0, count: 0, distance: null, quote: "", by: "" }),
      rating_avg: null,
      top_quote: null,
      status: "pending",
      hours: (body.hours as PlaceDetail["hours"]) ?? null,
      rating_distribution: { "1": 0, "2": 0, "3": 0, "4": 0, "5": 0 },
    };
    places.push(place);
    return { place };
  }

  if ((m = path.match(/^\/api\/places\/([^/]+)$/)) && method === "GET") {
    const place = places.find((p) => p.id === m![1]) ?? pendingPlaces.find((p) => p.id === m![1]);
    if (!place || (place.status !== "active" && me?.role !== "admin")) return fail(404, "not_found", "We couldn't find that place.");
    const mine = me ? reviews.find((r) => r.place_id === place.id && r.id.endsWith("mine")) : undefined;
    return { place: { ...place, viewer_review_id: mine?.id ?? null } };
  }

  if ((m = path.match(/^\/api\/places\/([^/]+)\/reviews$/))) {
    const placeId = m[1];
    if (method === "GET") {
      const sort = query.sort ?? "helpful";
      const list = reviews.filter((r) => r.place_id === placeId).map((r) => viewReview(r, me));
      list.sort((a, b) =>
        sort === "new"
          ? b.created_at.localeCompare(a.created_at)
          : sort === "lowest"
            ? a.rating - b.rating || b.created_at.localeCompare(a.created_at)
            : b.helpful_count - a.helpful_count || b.created_at.localeCompare(a.created_at),
      );
      return paginate(list, query);
    }
    if (method === "POST") {
      const user = requireMe(cookie, true);
      const existing = reviews.find((r) => r.place_id === placeId && r.id.endsWith("mine"));
      if (existing) fail(409, "already_reviewed", "You've already reviewed this place. Edit your review instead.", undefined, { review_id: existing.id });
      const fields: Record<string, string> = {};
      const rating = Number(body.rating);
      if (!(rating >= 1 && rating <= 5)) fields.rating = "Pick a rating from 1 to 5 stars.";
      const text = String(body.body ?? "").trim();
      if (text.length < 50) fields.body = "Write at least 50 characters so it's useful.";
      if (text.length > 2000) fields.body = "Keep it under 2,000 characters.";
      if (Object.keys(fields).length) fail(422, "validation", "Check the highlighted fields.", fields);
      const now = new Date().toISOString();
      const review: Review = {
        id: `${rid(Date.now() % 1e9).slice(0, -4)}mine`,
        place_id: placeId,
        rating: rating as Review["rating"],
        body: text,
        visited_month: (body.visited_month as string) ?? null,
        used_deal: !!body.used_deal,
        helpful_count: 0,
        viewer_voted: false,
        is_mine: true,
        author: author(user.display_name, user.school?.name ?? "UCLA"),
        created_at: now,
        updated_at: now,
      };
      reviews = [review, ...reviews];
      return { review };
    }
  }

  if ((m = path.match(/^\/api\/reviews\/([^/]+)$/))) {
    const review = reviews.find((r) => r.id === m![1]);
    if (!review) return fail(404, "not_found", "We couldn't find that review.");
    requireMe(cookie, true);
    if (method === "DELETE") {
      reviews = reviews.filter((r) => r.id !== review.id);
      deletedReviews.set(review.id, review);
      return undefined;
    }
    if (method === "PATCH") {
      Object.assign(review, body, { updated_at: new Date().toISOString() });
      return { review };
    }
  }

  if ((m = path.match(/^\/api\/reviews\/([^/]+)\/restore$/)) && method === "POST") {
    const review = deletedReviews.get(m[1]);
    if (!review) return fail(404, "not_found", "That review can't be restored anymore.");
    deletedReviews.delete(m[1]);
    reviews = [review, ...reviews];
    return { review };
  }

  if ((m = path.match(/^\/api\/reviews\/([^/]+)\/vote$/))) {
    requireMe(cookie, true);
    const review = reviews.find((r) => r.id === m![1]);
    if (!review) return fail(404, "not_found", "We couldn't find that review.");
    if (review.id.endsWith("mine")) fail(403, "forbidden", "You can't vote on your own review.");
    const want = method === "PUT";
    if (review.viewer_voted !== want) {
      review.viewer_voted = want;
      review.helpful_count += want ? 1 : -1;
    }
    return { helpful_count: review.helpful_count, viewer_voted: review.viewer_voted };
  }

  if (method === "POST" && path === "/api/reports") {
    requireMe(cookie, true);
    return { id: "c0ffee00-0000-4000-8000-0000000000aa" };
  }

  /* Admin */
  if (method === "GET" && path === "/api/admin/domains") {
    requireAdmin(cookie);
    return { items: domains.filter((d) => d.status === (query.status ?? "pending")) };
  }
  if ((m = path.match(/^\/api\/admin\/domains\/([^/]+)\/(approve|reject|reset)$/)) && method === "POST") {
    requireAdmin(cookie);
    const domain = decodeURIComponent(m[1]);
    const item = domains.find((d) => d.domain === domain);
    if (!item) return fail(404, "not_found", "We couldn't find that domain.");
    const next: AdminDomain = {
      ...item,
      status: m[2] === "approve" ? "approved" : m[2] === "reject" ? "rejected" : "pending",
      decided_at: m[2] === "reset" ? null : new Date().toISOString(),
    };
    domains = domains.map((d) => (d.domain === domain ? next : d));
    return next;
  }
  if (method === "GET" && path === "/api/admin/places") {
    requireAdmin(cookie);
    return paginate(pendingPlaces.filter((p) => p.status === "pending"), query);
  }
  if ((m = path.match(/^\/api\/admin\/places\/([^/]+)\/(approve|hide)$/)) && method === "POST") {
    requireAdmin(cookie);
    const place = pendingPlaces.find((p) => p.id === m![1]) ?? places.find((p) => p.id === m![1]);
    if (!place) return fail(404, "not_found", "We couldn't find that place.");
    place.status = m[2] === "approve" ? "active" : "hidden";
    return { place };
  }
  if (method === "GET" && path === "/api/admin/reports") {
    requireAdmin(cookie);
    return paginate(reports.filter((r) => r.status === (query.status ?? "open")), query);
  }
  if ((m = path.match(/^\/api\/admin\/reports\/([^/]+)\/resolve$/)) && method === "POST") {
    requireAdmin(cookie);
    reports = reports.map((r) => (r.id === m![1] ? { ...r, status: "resolved" } : r));
    return { ok: true };
  }

  return fail(404, "not_found", `No mock for ${method} ${path}.`);
}

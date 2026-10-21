/** Enrollment month rules from docs/api.md: access lasts 5 years from enrolled_on. */

export const ACCESS_YEARS = 5;
export const ENROLLMENT_TOO_OLD = "Corner is for current students. Access lasts 5 years from when you started.";

/** Years offered in the "When did you start?" Select, newest first. */
export function enrollmentYears(now: Date): number[] {
  const y = now.getFullYear();
  const out: number[] = [];
  for (let year = y + 1; year >= y - ACCESS_YEARS; year--) out.push(year);
  return out;
}

/** An error message for an enrollment month, or null when it's acceptable. */
export function enrollmentError(month: string, year: string, now: Date): string | null {
  if (!month || !year) return "Pick the month and year you started.";
  const start = Number(year) * 12 + (Number(month) - 1);
  const current = now.getFullYear() * 12 + now.getMonth();
  if (start > current + 12) return "Pick a month within the next year.";
  // access_until is the first day of enrolled_on + 5 years; it must still be ahead.
  if (start + ACCESS_YEARS * 12 <= current) return ENROLLMENT_TOO_OLD;
  return null;
}

export const toEnrolledOn = (month: string, year: string) => `${year}-${month.padStart(2, "0")}`;

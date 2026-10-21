import type { HoursRow } from "@/lib/api/types";

const DAYS = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/** Which weekdays (0 = Sunday) a free-text days label like "Mon – Thu" covers. */
export function daysCovered(label: string): Set<number> {
  const out = new Set<number>();
  const text = label.toLowerCase().trim();
  if (/daily|every ?day/.test(text)) return new Set([0, 1, 2, 3, 4, 5, 6]);
  if (/weekdays/.test(text)) [1, 2, 3, 4, 5].forEach((d) => out.add(d));
  if (/weekends/.test(text)) [0, 6].forEach((d) => out.add(d));
  for (const part of text.split(/,|&|\band\b/)) {
    const range = part.split(/\s*(?:–|—|-|\bto\b)\s*/).map((s) => DAYS.indexOf(s.trim().slice(0, 3)));
    if (range.length === 2 && range[0] >= 0 && range[1] >= 0) {
      for (let d = range[0], i = 0; i < 7; d = (d + 1) % 7, i++) {
        out.add(d);
        if (d === range[1]) break;
      }
    } else if (range.length === 1 && range[0] >= 0) out.add(range[0]);
  }
  return out;
}

function parseClock(s: string): number | null {
  const m = s.trim().toLowerCase().match(/^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/);
  if (!m) {
    if (/noon/.test(s)) return 12 * 60;
    if (/midnight/.test(s)) return 24 * 60;
    return null;
  }
  let h = Number(m[1]) % 12;
  if (m[3] === "pm") h += 12;
  if (!m[3] && Number(m[1]) === 12) h = 12;
  return h * 60 + Number(m[2] ?? 0);
}

/** Opening and closing minutes after midnight; close may exceed 24h for late nights. */
export function timeRange(label: string): { open: number; close: number; closeText: string; openText: string } | null {
  const text = label.trim();
  if (/24 ?h/i.test(text)) return { open: 0, close: 24 * 60, openText: "12 am", closeText: "midnight" };
  const parts = text.split(/\s*(?:–|—|-|\bto\b)\s*/);
  if (parts.length !== 2) return null;
  const open = parseClock(parts[0].includes("m") || !parts[1] ? parts[0] : `${parts[0]} ${parts[1].replace(/[^apm]/gi, "")}`);
  const close = parseClock(parts[1]);
  if (open === null || close === null) return null;
  return { open, close: close <= open ? close + 24 * 60 : close, openText: parts[0].trim(), closeText: parts[1].trim() };
}

export function todayRowIndex(hours: HoursRow[], now: Date): number {
  return hours.findIndex((h) => daysCovered(h.days).has(now.getDay()));
}

/** "Open until 10 pm", "Opens at 11 am", "Closed today", or null when the hours can't be read. */
export function openStatus(hours: HoursRow[] | null, now: Date): { text: string; open: boolean } | null {
  if (!hours?.length) return null;
  const i = todayRowIndex(hours, now);
  if (i < 0) return null;
  const row = hours[i];
  if (/closed/i.test(row.time)) return { text: "Closed today", open: false };
  const range = timeRange(row.time);
  if (!range) return null;
  const minutes = now.getHours() * 60 + now.getMinutes();
  if (minutes >= range.open && minutes < range.close) return { text: `Open until ${range.closeText}`, open: true };
  if (minutes < range.open) return { text: `Opens at ${range.openText}`, open: false };
  return { text: "Closed now", open: false };
}

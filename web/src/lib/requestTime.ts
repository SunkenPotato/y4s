import "server-only";
import { connection } from "next/server";

/** The time of this request, for server-rendering countdowns that the client then keeps live. */
export async function requestTime(): Promise<number> {
  await connection();
  return Date.now();
}

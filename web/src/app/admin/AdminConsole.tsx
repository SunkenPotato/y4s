"use client";

import Link from "next/link";
import { useState } from "react";
import { Badge } from "@/components/Badge/Badge";
import { Button } from "@/components/Button/Button";
import { EmptyState } from "@/components/EmptyState/EmptyState";
import { Muted, PageHead, TextLink } from "@/components/Layout/Layout";
import { Segmented } from "@/components/Segmented/Segmented";
import { useToast } from "@/components/Toast/Toast";
import { api, isApiError } from "@/lib/api/client";
import type { AdminDomain, AdminPlace, AdminReport, Page, ReportReason } from "@/lib/api/types";
import { baseDomain, placeMeta, plural, schoolPlace, shortDate } from "@/lib/format";
import { useCurrentYear } from "@/lib/hooks/useCurrentYear";
import styles from "./admin.module.css";

type Tab = "domains" | "places" | "reports";

const REASON_LABEL: Record<ReportReason, string> = {
  spam: "Spam or advertising",
  offensive: "Offensive or harassing",
  not_a_student: "Not written by a student",
  wrong_info: "Wrong information",
  other: "Something else",
};

interface AdminConsoleProps {
  pending: AdminDomain[];
  decided: AdminDomain[];
  places: Page<AdminPlace>;
  reports: Page<AdminReport>;
}

export function AdminConsole(props: AdminConsoleProps) {
  const toast = useToast();
  const year = useCurrentYear();
  const [tab, setTab] = useState<Tab>("domains");
  const [pending, setPending] = useState(props.pending);
  const [decided, setDecided] = useState(props.decided);
  const [places, setPlaces] = useState(props.places.items);
  const [reports, setReports] = useState(props.reports.items);
  const [busy, setBusy] = useState<string | null>(null);

  const fail = (err: unknown, fallback: string) => toast.show({ message: isApiError(err) ? err.message : fallback });

  /* Domains: decisions apply right away; Undo calls reset. */

  async function decideDomain(item: AdminDomain, action: "approve" | "reject") {
    setBusy(item.domain);
    try {
      const updated =
        action === "approve"
          ? await api.approveDomain(item.domain)
          : await api.rejectDomain(
              item.domain,
              item.flags.includes("personal_provider")
                ? "This is a personal email provider, not a school address."
                : `We couldn't confirm this domain belongs to ${item.school.name}.`,
            );
      setPending((list) => list.filter((d) => d.domain !== item.domain));
      setDecided((list) => [updated, ...list.filter((d) => d.domain !== item.domain)]);
      toast.show({
        message:
          action === "approve"
            ? `Approved @${item.domain}${item.waiting_count ? `. ${plural(item.waiting_count, "student")} can post now.` : ""}`
            : `Rejected @${item.domain}`,
        onUndo: () => resetDomain(updated),
      });
    } catch (err) {
      fail(err, "Couldn't save that decision. Try again.");
    } finally {
      setBusy(null);
    }
  }

  async function resetDomain(item: AdminDomain) {
    try {
      const restored = await api.resetDomain(item.domain);
      setDecided((list) => list.filter((d) => d.domain !== item.domain));
      setPending((list) => [restored, ...list.filter((d) => d.domain !== item.domain)]);
    } catch (err) {
      fail(err, "Couldn't undo. Try again.");
    }
  }

  /* Places and reports: no undo endpoint, so the action waits until the toast closes. */

  function decidePlace(item: AdminPlace, action: "approve" | "hide") {
    const index = places.findIndex((p) => p.id === item.id);
    setPlaces((list) => list.filter((p) => p.id !== item.id));
    toast.show({
      message: action === "approve" ? `Approved ${item.name}` : `Rejected ${item.name}`,
      onUndo: () => setPlaces((list) => [...list.slice(0, index), item, ...list.slice(index)]),
      onCommit: async () => {
        try {
          if (action === "approve") await api.approvePlace(item.id);
          else await api.hidePlace(item.id);
        } catch (err) {
          setPlaces((list) => [...list.slice(0, index), item, ...list.slice(index)]);
          fail(err, `Couldn't save the decision on ${item.name}.`);
        }
      },
    });
  }

  function resolveReport(item: AdminReport, action: "dismiss" | "remove_content") {
    const index = reports.findIndex((r) => r.id === item.id);
    // remove_content resolves every open report on the same target.
    const affected = action === "remove_content" ? reports.filter((r) => r.target_id === item.target_id) : [item];
    setReports((list) => list.filter((r) => !affected.some((a) => a.id === r.id)));
    const restore = () =>
      setReports((list) => {
        const rest = list.filter((r) => !affected.some((a) => a.id === r.id));
        return [...rest.slice(0, index), ...affected, ...rest.slice(index)];
      });
    toast.show({
      message: action === "dismiss" ? "Report dismissed" : item.target_type === "review" ? "Review removed" : "Place hidden",
      onUndo: restore,
      onCommit: async () => {
        try {
          await api.resolveReport(item.id, action);
        } catch (err) {
          restore();
          fail(err, "Couldn't resolve that report.");
        }
      },
    });
  }

  const tabs = [
    { value: "domains", label: `Domains · ${pending.length}` },
    { value: "places", label: `New places · ${places.length}${props.places.next_cursor ? "+" : ""}` },
    { value: "reports", label: `Reports · ${reports.length}${props.reports.next_cursor ? "+" : ""}` },
  ] as const;

  const heads: Record<Tab, { title: string; lede: string }> = {
    domains: {
      title: "School domains",
      lede: "Approving a domain lets everyone with that email sign up for the school right away.",
    },
    places: { title: "New places", lede: "Students added these. Approve the ones that are real and near a school." },
    reports: { title: "Reports", lede: "Removing content hides it everywhere and closes every report on it." },
  };

  return (
    <div className={styles.screen}>
      <PageHead title={heads[tab].title} size="title">
        {heads[tab].lede}
      </PageHead>

      <Segmented label="Queues" mode="tabs" idPrefix="admin" options={tabs} value={tab} onChange={setTab} className={styles.tabs} />

      <div role="tabpanel" id={`admin-panel-${tab}`} aria-labelledby={`admin-tab-${tab}`} className={styles.panel}>
        {tab === "domains" ? (
          <>
            {pending.length === 0 ? (
              <EmptyState icon="school" title="No domains waiting">
                New school email domains show up here when a student requests access.
              </EmptyState>
            ) : (
              <div className={styles.list}>
                {pending.map((d) => {
                  const personal = d.flags.includes("personal_provider");
                  const base = baseDomain(d.domain);
                  return (
                    <article key={d.domain} className={styles.card} aria-labelledby={`domain-${d.domain}`}>
                      <div className={styles.info}>
                        <div className={styles.titleRow}>
                          <h2 id={`domain-${d.domain}`} className={styles.cardTitle}>
                            @{d.domain}
                          </h2>
                          <Badge tone="pending">{d.waiting_count} waiting</Badge>
                        </div>
                        <p className={styles.line}>
                          Claimed for <strong>{d.school.name}</strong> · {schoolPlace(d.school)}
                        </p>
                        <Muted small>
                          {personal ? "Personal email provider, cannot belong to a school" : `Matches ${base}`} · First requested{" "}
                          {shortDate(d.requested_at, year)}
                        </Muted>
                      </div>
                      <div className={styles.actions}>
                        {!personal ? (
                          <a href={`https://${base}`} target="_blank" rel="noopener noreferrer" className={styles.link}>
                            Check website
                          </a>
                        ) : null}
                        <Button variant="danger" size={44} onClick={() => decideDomain(d, "reject")} disabled={busy === d.domain}>
                          Reject
                        </Button>
                        <Button size={44} onClick={() => decideDomain(d, "approve")} loading={busy === d.domain} loadingLabel="Saving…">
                          Approve
                        </Button>
                      </div>
                    </article>
                  );
                })}
              </div>
            )}

            {decided.length ? (
              <section aria-labelledby="decided-title" className={styles.decided}>
                <h2 id="decided-title" className={styles.subhead}>
                  Recently decided
                </h2>
                <ul className={styles.decidedList}>
                  {decided.map((d) => (
                    <li key={d.domain} className={styles.decidedRow}>
                      <span className={styles.decidedDomain}>@{d.domain}</span>
                      <Muted as="span">{d.school.name}</Muted>
                      <Badge tone={d.status === "approved" ? "verified" : "rejected"}>
                        {d.status === "approved" ? "Approved" : "Rejected"}
                      </Badge>
                      <TextLink onClick={() => resetDomain(d)}>Undo</TextLink>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
          </>
        ) : null}

        {tab === "places" ? (
          places.length === 0 ? (
            <EmptyState icon="pin" title="No new places waiting">
              Places students add show up here before anyone else sees them.
            </EmptyState>
          ) : (
            <div className={styles.list}>
              {places.map((p) => (
                <article key={p.id} className={styles.card} aria-labelledby={`place-${p.id}`}>
                  <div className={styles.info}>
                    <div className={styles.titleRow}>
                      <h2 id={`place-${p.id}`} className={styles.cardTitle}>
                        <Link href={`/places/${p.id}`}>{p.name}</Link>
                      </h2>
                      {p.student_discount ? <Badge tone="deal">{p.student_discount}</Badge> : null}
                    </div>
                    <p className={styles.line}>
                      {placeMeta(p)} · {p.address}
                    </p>
                    <Muted small>
                      Added by {p.created_by.display_name}, {p.created_by.school_name}
                      {p.school ? ` · Linked to ${p.school.name}` : ""}
                    </Muted>
                  </div>
                  <div className={styles.actions}>
                    <Button variant="danger" size={44} onClick={() => decidePlace(p, "hide")}>
                      Reject
                    </Button>
                    <Button size={44} onClick={() => decidePlace(p, "approve")}>
                      Approve
                    </Button>
                  </div>
                </article>
              ))}
            </div>
          )
        ) : null}

        {tab === "reports" ? (
          reports.length === 0 ? (
            <EmptyState icon="check" title="No open reports">
              Reports from students show up here.
            </EmptyState>
          ) : (
            <div className={styles.list}>
              {reports.map((r) => (
                <article key={r.id} className={styles.card} aria-labelledby={`report-${r.id}`}>
                  <div className={styles.info}>
                    <div className={styles.titleRow}>
                      <h2 id={`report-${r.id}`} className={styles.cardTitle}>
                        {REASON_LABEL[r.reason]}
                      </h2>
                      <Badge tone="neutral">{r.target_type === "review" ? "Review" : "Place"}</Badge>
                      {r.report_count > 1 ? <Badge tone="pending">{plural(r.report_count, "report")}</Badge> : null}
                    </div>
                    <p className={styles.line}>
                      {r.target_type === "place" ? (
                        <Link href={`/places/${r.target_id}`}>{r.target_preview}</Link>
                      ) : (
                        <>“{r.target_preview}”</>
                      )}
                    </p>
                    {r.details ? <p className={styles.line}>{r.details}</p> : null}
                    <Muted small>
                      Reported by {r.reporter.display_name}, {r.reporter.school_name} · {shortDate(r.created_at, year)}
                    </Muted>
                  </div>
                  <div className={styles.actions}>
                    <Button variant="secondary" size={44} onClick={() => resolveReport(r, "dismiss")}>
                      Dismiss
                    </Button>
                    <Button variant="danger" size={44} onClick={() => resolveReport(r, "remove_content")}>
                      {r.target_type === "review" ? "Remove review" : "Hide place"}
                    </Button>
                  </div>
                </article>
              ))}
            </div>
          )
        ) : null}
      </div>
    </div>
  );
}

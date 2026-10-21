"use client";

import { useRouter } from "next/navigation";
import { useRef, useState } from "react";
import { Button } from "@/components/Button/Button";
import { Icon } from "@/components/Icon/Icon";
import { Row } from "@/components/Layout/Layout";
import { useToast } from "@/components/Toast/Toast";
import { api, isApiError } from "@/lib/api/client";
import { loginHref } from "@/lib/auth";
import { useNow } from "@/lib/hooks/useNow";
import { ReportDialog } from "./ReportDialog";
import styles from "./place.module.css";

interface PlaceActionsProps {
  place: {
    id: string;
    name: string;
    lat: number;
    lng: number;
    viewer_review_id: string | null;
    viewer_saved: boolean;
    status: string;
  };
  signedIn: boolean;
  /** editable_until of the viewer's own review, if they have one. */
  ownReviewEditableUntil: string | null;
  /** Server render time, so the edit label is right before hydration. */
  renderedAt: number;
}

/** Write (or edit/see) a review, Save, Directions, and Report. */
export function PlaceActions({ place, signedIn, ownReviewEditableUntil, renderedAt }: PlaceActionsProps) {
  const router = useRouter();
  const toast = useToast();
  const now = useNow(1000, renderedAt);
  const [reporting, setReporting] = useState(false);
  const [saved, setSaved] = useState(place.viewer_saved);
  const latest = useRef(0);
  const here = `/places/${place.id}`;
  const directions = `https://www.openstreetmap.org/directions?to=${place.lat}%2C${place.lng}`;
  const editable = !!ownReviewEditableUntil && now !== null && new Date(ownReviewEditableUntil).getTime() > now;

  // Optimistic, no toast on success. The last click wins if several are in flight.
  async function toggleSave() {
    if (!signedIn) {
      router.push(loginHref(here));
      return;
    }
    const want = !saved;
    const seq = ++latest.current;
    setSaved(want);
    try {
      const res = want ? await api.save(place.id) : await api.unsave(place.id);
      if (seq === latest.current) setSaved(res.saved);
    } catch (e) {
      if (seq !== latest.current) return;
      setSaved(!want);
      toast.show({ message: isApiError(e) ? e.message : "Couldn't update your saved places. Try again." });
    }
  }

  return (
    <Row gap={2}>
      {place.viewer_review_id ? (
        editable ? (
          <Button href={`${here}/review?edit=${place.viewer_review_id}`}>Edit your review</Button>
        ) : (
          <Button href={`#review-${place.viewer_review_id}`}>See your review</Button>
        )
      ) : place.status === "active" ? (
        <Button href={`${here}/review`}>Write a review</Button>
      ) : null}
      <Button
        variant="secondary"
        aria-pressed={saved}
        onClick={toggleSave}
        icon={<Icon name="bookmark" size={18} filled={saved} className={saved ? styles.savedIcon : undefined} />}
      >
        {saved ? "Saved" : "Save"}
      </Button>
      <Button
        variant="secondary"
        href={directions}
        target="_blank"
        rel="noopener noreferrer"
        icon={<Icon name="directions" size={18} />}
      >
        Directions
      </Button>
      <Button variant="ghost" onClick={() => (signedIn ? setReporting(true) : router.push(loginHref(here)))}>
        Report
      </Button>
      <ReportDialog
        open={reporting}
        onClose={() => setReporting(false)}
        target={{ type: "place", id: place.id, name: place.name }}
      />
    </Row>
  );
}

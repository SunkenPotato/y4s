"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { Button } from "@/components/Button/Button";
import { Icon } from "@/components/Icon/Icon";
import { Row } from "@/components/Layout/Layout";
import { loginHref } from "@/lib/auth";
import { ReportDialog } from "./ReportDialog";

interface PlaceActionsProps {
  place: { id: string; name: string; lat: number; lng: number; viewer_review_id: string | null; status: string };
  signedIn: boolean;
}

/** Write a review (or see yours), Directions, and Report. */
export function PlaceActions({ place, signedIn }: PlaceActionsProps) {
  const router = useRouter();
  const [reporting, setReporting] = useState(false);
  const directions = `https://www.openstreetmap.org/directions?to=${place.lat}%2C${place.lng}`;

  return (
    <Row gap={2}>
      {place.viewer_review_id ? (
        <Button href={`#review-${place.viewer_review_id}`}>See your review</Button>
      ) : place.status === "active" ? (
        <Button href={`/places/${place.id}/review`}>Write a review</Button>
      ) : null}
      <Button
        variant="secondary"
        href={directions}
        target="_blank"
        rel="noopener noreferrer"
        icon={<Icon name="directions" size={18} />}
      >
        Directions
      </Button>
      <Button
        variant="ghost"
        onClick={() => (signedIn ? setReporting(true) : router.push(loginHref(`/places/${place.id}`)))}
      >
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

"use client";

import dynamic from "next/dynamic";
import styles from "./Map.module.css";

export interface MapPoint {
  id: string;
  lat: number;
  lng: number;
  label: string;
  href?: string;
}

export interface MapProps {
  points: MapPoint[];
  /** Used when there are no points. */
  center?: { lat: number; lng: number };
  zoom?: number;
  height?: number;
  /** Accessible name for the map region. */
  label: string;
  /** Text under the map, like the address. */
  caption?: string;
}

const LeafletMap = dynamic(() => import("./LeafletMap"), {
  ssr: false,
  loading: () => <div className={styles.loading} aria-hidden="true" />,
});

/** Leaflet with OpenStreetMap tiles inside a surface tile. Pins are accent squares with a bg ring. */
export function Map({ caption, height = 180, ...props }: MapProps) {
  return (
    <figure className={styles.tile}>
      <div className={styles.frame} style={{ height }}>
        <LeafletMap {...props} />
      </div>
      {caption ? <figcaption className={styles.caption}>{caption}</figcaption> : null}
    </figure>
  );
}

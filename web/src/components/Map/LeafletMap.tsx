"use client";

import "leaflet/dist/leaflet.css";
import L from "leaflet";
import Link from "next/link";
import { useEffect } from "react";
import { MapContainer, Marker, Popup, TileLayer, useMap } from "react-leaflet";
import type { MapProps } from "./Map";
import styles from "./Map.module.css";

const pin = L.divIcon({
  className: styles.pin,
  iconSize: [16, 16],
  iconAnchor: [8, 8],
  popupAnchor: [0, -10],
});

function FitBounds({ points }: Pick<MapProps, "points">) {
  const map = useMap();
  useEffect(() => {
    if (points.length > 1) {
      map.fitBounds(L.latLngBounds(points.map((p) => [p.lat, p.lng])), { padding: [24, 24], maxZoom: 16 });
    }
  }, [map, points]);
  return null;
}

export default function LeafletMap({ points, center, zoom = 15, label }: MapProps) {
  const first = points[0] ?? center ?? { lat: 34.0689, lng: -118.4452 };
  return (
    <MapContainer
      center={[first.lat, first.lng]}
      zoom={zoom}
      scrollWheelZoom={false}
      className={styles.map}
      attributionControl
      aria-label={label}
    >
      <TileLayer
        url="https://tile.openstreetmap.org/{z}/{x}/{y}.png"
        attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
        maxZoom={19}
      />
      {points.map((p) => (
        <Marker key={p.id} position={[p.lat, p.lng]} icon={pin} title={p.label} alt={p.label} keyboard>
          {p.href ? (
            <Popup>
              <Link href={p.href} className={styles.popupLink}>
                {p.label}
              </Link>
            </Popup>
          ) : null}
        </Marker>
      ))}
      <FitBounds points={points} />
    </MapContainer>
  );
}

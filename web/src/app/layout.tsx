import type { Metadata, Viewport } from "next";
import type React from "react";
import { Schibsted_Grotesk } from "next/font/google";
import { ToastProvider } from "@/components/Toast/Toast";
import "@/styles/tokens.css";
import "./globals.css";

const schibsted = Schibsted_Grotesk({
  subsets: ["latin"],
  weight: ["400", "500", "600", "700", "800"],
  variable: "--font-schibsted",
  display: "swap",
});

export const metadata: Metadata = {
  title: { default: "Corner", template: "%s · Corner" },
  description: "Places near campus, reviewed by verified students.",
};

export const viewport: Viewport = {
  colorScheme: "light dark",
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={schibsted.variable}>
      <body>
        <ToastProvider>{children}</ToastProvider>
      </body>
    </html>
  );
}

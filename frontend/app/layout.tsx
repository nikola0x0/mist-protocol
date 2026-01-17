import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "./providers";
import { Tektur, Inter } from "next/font/google";

const tektur = Tektur({
  subsets: ["latin"],
  variable: "--font-tektur",
});

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
});

export const metadata: Metadata = {
  metadataBase: new URL('https://0xmist.xyz'),
  title: {
    default: "Mist Protocol",
    template: "%s | Mist Protocol"
  },
  description: "Privacy-preserving intent-based DeFi on Sui. Trade, swap, and transact with complete privacy using zero-knowledge proofs and secure multi-party computation.",
  keywords: ["DeFi", "Sui", "Privacy", "Zero-Knowledge", "Blockchain", "Cryptocurrency", "Private Trading", "MPC", "Intent-based Trading"],
  authors: [{ name: "Mist Protocol Team" }],
  creator: "Mist Protocol",
  publisher: "Mist Protocol",
  openGraph: {
    type: "website",
    locale: "en_US",
    url: "https://0xmist.xyz",
    siteName: "Mist Protocol",
    title: "Mist Protocol - Private DeFi on Sui",
    description: "Privacy-preserving intent-based DeFi on Sui. Trade, swap, and transact with complete privacy using zero-knowledge proofs and secure multi-party computation.",
    images: [
      {
        url: "/assets/og-image.png",
        width: 1200,
        height: 630,
        alt: "Mist Protocol - Private DeFi on Sui",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Mist Protocol - Private DeFi on Sui",
    description: "Privacy-preserving intent-based DeFi on Sui. Trade, swap, and transact with complete privacy.",
    images: ["/assets/og-image.png"],
    creator: "@mistprotocol",
    site: "@mistprotocol",
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      'max-video-preview': -1,
      'max-image-preview': 'large',
      'max-snippet': -1,
    },
  },
  icons: {
    icon: [
      { url: "/favicon.ico" },
      { url: "/favicon-96x96.png", sizes: "96x96", type: "image/png" },
      { url: "/favicon.svg", type: "image/svg+xml" },
    ],
    shortcut: "/favicon.ico",
    apple: { url: "/apple-touch-icon.png", sizes: "180x180" },
  },
  manifest: "/site.webmanifest",
  appleWebApp: {
    capable: true,
    title: "Mist Protocol",
    statusBarStyle: "black-translucent",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${tektur.variable} ${inter.variable}`}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}

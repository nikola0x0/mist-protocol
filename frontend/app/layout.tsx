import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "./providers";
import { Tektur, Anonymous_Pro } from "next/font/google";

const tektur = Tektur({
  subsets: ["latin"],
  variable: "--font-tektur",
});

const anonymousPro = Anonymous_Pro({
  weight: ["400", "700"],
  subsets: ["latin"],
  variable: "--font-anonymous-pro",
});

export const metadata: Metadata = {
  title: "Mist Protocol - Private DeFi on Sui",
  description: "Privacy-preserving intent-based DeFi on Sui",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${tektur.variable} ${anonymousPro.variable}`}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}

"use client";

import Image from "next/image";
import Link from "next/link";
import { useEffect, useRef } from "react";

export default function Landing() {
  const lettersRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const chars =
      "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    const randomChar = () =>
      chars[Math.floor(Math.random() * (chars.length - 1))];
    const randomString = (length: number) =>
      Array.from(Array(length)).map(randomChar).join("");

    const letters = lettersRef.current;

    if (!letters) return;

    const handleOnMove = (e: MouseEvent | TouchEvent) => {
      const clientX = "clientX" in e ? e.clientX : e.touches[0].clientX;
      const clientY = "clientY" in e ? e.clientY : e.touches[0].clientY;

      letters.style.setProperty("--x", `${clientX}px`);
      letters.style.setProperty("--y", `${clientY}px`);
      letters.innerText = randomString(20000);
    };

    const onMouseMove = (e: MouseEvent) => handleOnMove(e);
    const onTouchMove = (e: TouchEvent) => handleOnMove(e);

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("touchmove", onTouchMove);

    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("touchmove", onTouchMove);
    };
  }, []);

  return (
    <div className="h-screen bg-black relative overflow-hidden flex flex-col">
      {/* Full Screen Hover Effect */}
      <div className="card-letters fixed inset-0 z-0" ref={lettersRef}></div>

      {/* Header */}
      <header className="relative z-30 backdrop-blur-lg">
        <div className="container mx-auto px-6 py-4 flex justify-between items-center">
          <div className="flex items-center gap-3">
            <Image
              src="/assets/logo.svg"
              alt="Mist Protocol"
              width={32}
              height={32}
              className="opacity-90"
            />
            <h1 className="text-xl font-tektur text-white">Mist Protocol</h1>
          </div>
          <Link href="/app">
            <button className="glass-button px-6 py-2.5 text-white hover:glow">
              launch app
            </button>
          </Link>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 flex flex-col items-center justify-center relative z-10 pointer-events-none">
        <div className="mb-8 pointer-events-auto">
          <Image
            src="/assets/logo.svg"
            alt="Mist Protocol"
            width={200}
            height={200}
            className="drop-shadow-[0_0_30px_rgba(255,255,255,0.1)]"
          />
        </div>

        {/* Text Content Below Logo */}
        <div className="max-w-4xl mx-auto text-center space-y-6 px-6 relative z-50 pointer-events-auto">
          <h2 className="text-6xl md:text-7xl font-bold leading-tight font-tektur animate-slide-up select-none">
            <span className="gradient-text">Mist Protocol</span>
          </h2>

          <p
            className="text-xl md:text-2xl text-gray-400 font-inter animate-slide-up"
            style={{ animationDelay: "0.1s" }}
          >
            A privacy layer for DeFi on Sui
          </p>
          <p
            className="text-lg text-gray-500 font-inter animate-slide-up"
            style={{ animationDelay: "0.2s" }}
          >
            To protect, your alpha
          </p>

          <div
            className="pt-4 animate-slide-up"
            style={{ animationDelay: "0.3s" }}
          >
            <Link href="/app">
              <button className="glass-button px-12 py-4 text-lg text-white hover:glow font-tektur">
                launch app
              </button>
            </Link>
          </div>
        </div>
      </main>

      {/* Footer */}
      <footer className="relative z-30 backdrop-blur-lg py-6">
        <div className="container mx-auto px-6 flex justify-end items-center">
          <div className="flex items-center gap-4">
            <a
              href="https://github.com/nikola0x0/mist-protocol"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-400 hover:text-white transition-colors"
              aria-label="GitHub"
            >
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
              </svg>
            </a>
            <a
              href="https://github.com/nikola0x0/mist-protocol/blob/main/README.md"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gray-400 hover:text-white transition-colors"
              aria-label="Documentation"
            >
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
            </a>
          </div>
        </div>
      </footer>
    </div>
  );
}

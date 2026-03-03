import type { Metadata } from "next";

/* ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ  TECHSTACK PAGE SEO  ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ */
export const metadata: Metadata = {
  title: "Live Tech Stack Map — Infinity Autonomous AI System | 154k+ LOC",
  description:
    "Interactive real-time architecture visualization of the Infinity autonomous AI system. 154,000+ LOC across Rust (Vitalis compiler with Cranelift JIT, 35,632 LOC, 748 tests + Nova LLM engine, from-scratch transformer with CUDA), Python (FastAPI, 72 AI modules, multi-agent swarm), and TypeScript (Next.js cyberpunk dashboard). Self-evolving code engine with Asimov's Laws safety governance. Built solo by Bart Chmiel.",
  alternates: { canonical: "/techstack" },
  keywords: [
    // ÔöÇÔöÇ Architecture & Visualization ÔöÇÔöÇ
    "AI architecture diagram",
    "AI architecture visualization",
    "live tech stack",
    "interactive tech stack map",
    "real-time AI architecture",
    "system architecture visualization",
    // ÔöÇÔöÇ Core AI ÔöÇÔöÇ
    "self-evolving AI system",
    "autonomous AI system",
    "autonomous code evolution",
    "self-modifying AI",
    "recursive self-improvement",
    // ÔöÇÔöÇ Vitalis ÔöÇÔöÇ
    "Vitalis compiler",
    "Vitalis programming language",
    "Cranelift JIT compiler",
    "custom programming language Rust",
    "JIT compiled language",
    // ÔöÇÔöÇ Multi-Agent ÔöÇÔöÇ
    "multi-agent AI swarm",
    "swarm intelligence",
    "AI consensus protocol",
    "multi-agent consensus",
    // ÔöÇÔöÇ Nova LLM ÔöÇÔöÇ
    "LLM from scratch Rust",
    "custom transformer architecture",
    "CUDA LLM training",
    "Nova LLM engine",
    "from-scratch language model",
    "BPE tokenizer Rust",
    // ÔöÇÔöÇ Backend ÔöÇÔöÇ
    "FastAPI AI backend",
    "Rust AI compiler",
    "Python AI system",
    // ÔöÇÔöÇ Safety ÔöÇÔöÇ
    "AI safety Asimov laws",
    "AI guardian system",
    "capability-based sandbox",
    // ÔöÇÔöÇ Features ÔöÇÔöÇ
    "cyberpunk AI dashboard",
    "real-time AI monitoring",
    "episodic AI memory",
    "AI code generation engine",
    "solo developer AI system",
    "154000 lines of code AI",
    "72 AI modules",
    "748 automated tests",
  ],
  openGraph: {
    title: "Live Tech Stack Map — Infinity Autonomous AI System",
    description:
      "Watch an autonomous AI system in real-time. 72 modules, 142 API endpoints, custom Vitalis compiler (35.6k Rust LOC, 748 tests), and self-evolving code. Built solo by Bart Chmiel.",
    url: "https://infinitytechstack.uk/techstack",
    type: "website",
    siteName: "Infinity Tech Stack",
    locale: "en_GB",
    images: [
      {
        url: "/opengraph-image",
        width: 1200,
        height: 630,
        alt: "Infinity Tech Stack ÔÇö Live Architecture Map with 72 AI Modules",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Live Tech Stack Map ÔÇö Infinity AI",
    description:
      "154k+ LOC autonomous AI with custom Vitalis compiler. 72 modules, 748 tests, Cranelift JIT. Live interactive map.",
    images: ["/opengraph-image"],
  },
};

export default function TechStackLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return <>{children}</>;
}
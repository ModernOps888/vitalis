import "./globals.css";
import type { Metadata, Viewport } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { Analytics } from "@vercel/analytics/next";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" });
const jetbrains = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono" });

/* ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ  SEO ÔÇö METADATA  ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ */
export const metadata: Metadata = {
  title: {
    default: "Infinity Tech Stack ÔÇö Autonomous Self-Evolving AI System | infinitytechstack.uk",
    template: "%s | Infinity Tech Stack",
  },
  description:
    "Infinity is an autonomous self-evolving AI system built by a solo founder with Rust, Python & TypeScript. Features a custom Vitalis compiler (Cranelift JIT, 35,632 LOC, 748 tests), Nova LLM engine (from-scratch transformer with CUDA), 72 AI modules, 142 API endpoints, multi-agent swarm consensus, episodic memory, and Asimov's Laws safety. 154,000+ total LOC. Live interactive tech stack map at infinitytechstack.uk.",
  keywords: [
    // ÔöÇÔöÇ Brand & Core ÔöÇÔöÇ
    "Infinity Tech Stack",
    "infinity techstack",
    "infinitytechstack",
    "infinitytechstack.uk",
    "Infinity AI",
    "Infinity autonomous AI",
    "Infinity self-evolving AI",
    // ÔöÇÔöÇ Vitalis Language ÔöÇÔöÇ
    "Vitalis language",
    "Vitalis compiler",
    "Vitalis programming language",
    "custom programming language Rust",
    "Cranelift JIT compiler",
    "self-evolving programming language",
    "code evolution engine",
    // ÔöÇÔöÇ AI Architecture ÔöÇÔöÇ
    "autonomous AI system",
    "self-evolving AI",
    "AI tech stack",
    "AI architecture diagram",
    "AI system architecture",
    "multi-agent AI swarm",
    "swarm intelligence system",
    "AI consensus protocol",
    "autonomous code generation",
    "AI code evolution",
    "self-modifying AI",
    "recursive self-improvement AI",
    // ÔöÇÔöÇ Technologies ÔöÇÔöÇ
    "Rust AI compiler",
    "FastAPI AI backend",
    "Next.js AI dashboard",
    "Python AI backend",
    "TypeScript frontend",
    "Cranelift code generation",
    "JIT compiled language",
    // ÔöÇÔöÇ Safety & Ethics ÔöÇÔöÇ
    "AI safety Asimov laws",
    "ethical AI system",
    "capability-based AI sandbox",
    "AI guardian system",
    // ÔöÇÔöÇ Features ÔöÇÔöÇ
    "live tech stack map",
    "interactive architecture visualization",
    "real-time AI monitoring",
    "AI dashboard cyberpunk",
    "episodic AI memory",
    "vector similarity search AI",
    "rate limiting AI",
    // ÔöÇÔöÇ Solo Founder / Indie ÔöÇÔöÇ
    "solo founder AI project",
    "solo developer AI system",
    "indie AI startup",
    "one-person AI company",
    "Bart Chmiel AI",
    "Bart Chmiel developer",
    // ÔöÇÔöÇ Consulting ÔöÇÔöÇ
    "AI consulting UK",
    "AI security audit",
    "AI architecture consulting",
    "Microsoft Purview implementation",
    "enterprise AI consulting",
  ],
  metadataBase: new URL("https://infinitytechstack.uk"),
  alternates: {
    canonical: "/techstack",
    languages: { "en-GB": "/techstack" },
  },
  openGraph: {
    title: "Infinity Tech Stack ÔÇö Autonomous Self-Evolving AI System",
    description:
      "An autonomous AI that writes, tests, deploys & evolves its own code. Custom Vitalis compiler (Rust/Cranelift JIT, 35.6k LOC, 748 tests), Nova LLM engine with CUDA, 72 AI modules, multi-agent swarm consensus. 154k+ LOC. Built solo. Live interactive tech stack map.",
    url: "https://infinitytechstack.uk/techstack",
    siteName: "Infinity Tech Stack",
    locale: "en_GB",
    type: "website",
    images: [
      {
        url: "/opengraph-image",
        width: 1200,
        height: 630,
        alt: "Infinity Tech Stack ÔÇö Autonomous Self-Evolving AI System Architecture",
        type: "image/png",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "Infinity Tech Stack ÔÇö Autonomous Self-Evolving AI",
    description:
      "Solo-built autonomous AI with custom Vitalis compiler (Cranelift JIT), Nova LLM engine, 72 modules, 748 tests. Self-evolving code engine with Asimov's Laws. 154k+ LOC. Live interactive tech stack.",
    images: ["/opengraph-image"],
  },
  robots: {
    index: true,
    follow: true,
    nocache: false,
    googleBot: {
      index: true,
      follow: true,
      noimageindex: false,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  icons: {
    icon: [
      { url: "/icon", sizes: "32x32", type: "image/png" },
    ],
    apple: [
      { url: "/apple-icon", sizes: "180x180", type: "image/png" },
    ],
  },
  category: "technology",
  creator: "Bart Chmiel",
  publisher: "Infinity Tech Stack",
  authors: [{ name: "Bart Chmiel", url: "https://www.linkedin.com/in/modern-workplace-tech365/" }],
  other: {
    "google-site-verification": "", // Add GSC verification code when obtained
    "msvalidate.01": "", // Add Bing Webmaster verification when obtained
  },
};

/* ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ  SEO ÔÇö VIEWPORT  ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ */
export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  maximumScale: 5,
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "#0a0a0f" },
    { media: "(prefers-color-scheme: light)", color: "#0a0a0f" },
  ],
  colorScheme: "dark",
};

/* ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ  JSON-LD STRUCTURED DATA  ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ */
const jsonLdOrganization = {
  "@context": "https://schema.org",
  "@type": "Organization",
  "@id": "https://infinitytechstack.uk/#organization",
  name: "Infinity Tech Stack",
  alternateName: ["Infinity AI", "InfinityTechStack", "Infinity Autonomous AI"],
  url: "https://infinitytechstack.uk",
  logo: {
    "@type": "ImageObject",
    "@id": "https://infinitytechstack.uk/#logo",
    url: "https://infinitytechstack.uk/infinity-logo.png",
    contentUrl: "https://infinitytechstack.uk/infinity-logo.png",
    caption: "Infinity Tech Stack Logo",
  },
  image: "https://infinitytechstack.uk/opengraph-image",
  description:
    "Autonomous self-evolving AI system with custom Vitalis compiler (35,632 LOC, 748 tests), Nova LLM engine, multi-agent swarm, and Asimov's Laws safety. 154k+ LOC. Built solo by Bart Chmiel.",
  foundingDate: "2025-01-01",
  numberOfEmployees: {
    "@type": "QuantitativeValue",
    value: 1,
  },
  founder: {
    "@type": "Person",
    "@id": "https://infinitytechstack.uk/#founder",
    name: "Bart Chmiel",
    url: "https://www.linkedin.com/in/modern-workplace-tech365/",
    jobTitle: "Founder & AI Engineer",
  },
  sameAs: [
    "https://www.linkedin.com/in/modern-workplace-tech365/",
    "https://github.com/ModernOps888",
    "https://github.com/ModernOps888/vitalis",
  ],
  contactPoint: {
    "@type": "ContactPoint",
    email: "b.chmiel20@gmail.com",
    contactType: "customer service",
    availableLanguage: ["English", "Polish"],
  },
  knowsAbout: [
    "Artificial Intelligence",
    "Autonomous Systems",
    "Compiler Design",
    "Rust Programming",
    "AI Safety",
    "Multi-Agent Systems",
    "Code Evolution",
  ],
  knowsLanguage: ["en", "pl"],
};

const jsonLdWebSite = {
  "@context": "https://schema.org",
  "@type": "WebSite",
  "@id": "https://infinitytechstack.uk/#website",
  name: "Infinity Tech Stack",
  alternateName: "Infinity AI System",
  url: "https://infinitytechstack.uk",
  description:
    "Live interactive tech stack map for an autonomous self-evolving AI system. Custom Vitalis compiler, Nova LLM engine, 72 modules, 748 tests. 154k+ LOC.",
  publisher: {
    "@id": "https://infinitytechstack.uk/#organization",
  },
  inLanguage: "en-GB",
  potentialAction: {
    "@type": "SearchAction",
    target: "https://infinitytechstack.uk/techstack?q={search_term_string}",
    "query-input": "required name=search_term_string",
  },
};

const jsonLdSoftwareApplication = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "Infinity ÔÇö Autonomous Self-Evolving AI System",
  alternateName: "Infinity AI",
  applicationCategory: "DeveloperApplication",
  applicationSubCategory: "Artificial Intelligence Platform",
  operatingSystem: "Linux, Windows",
  description:
    "An autonomous AI system that writes, tests, deploys and evolves its own code. Features a custom Vitalis compiler (35,632 LOC, 748 tests) and Nova LLM engine built in Rust with Cranelift JIT and CUDA, multi-agent swarm consensus with Asimov's Laws safety, episodic memory, and 72 AI modules across 154,000+ lines of code.",
  url: "https://infinitytechstack.uk/techstack",
  author: {
    "@id": "https://infinitytechstack.uk/#founder",
  },
  programmingLanguage: ["Rust", "Python", "TypeScript", "Vitalis"],
  runtimePlatform: "Cranelift JIT",
  softwareVersion: "20.0",
  datePublished: "2025-01-01",
  dateModified: new Date().toISOString().split("T")[0],
  featureList: [
    "Custom Vitalis Compiler (Cranelift JIT, 35,632 Rust LOC)",
    "748 Automated Tests",
    "72 AI Modules",
    "142 API Endpoints",
    "Multi-Agent Swarm Consensus",
    "Episodic Memory System",
    "Asimov's Laws Guardian",
    "Self-Evolving Code Engine",
    "44 Native Hotpath Operations",
    "318 Python API Bindings",
    "405 FFI Exports",
    "Real-Time Monitoring Dashboard",
  ],
  screenshot: "https://infinitytechstack.uk/opengraph-image",
  offers: {
    "@type": "Offer",
    price: "0",
    priceCurrency: "GBP",
    availability: "https://schema.org/InStock",
    description: "Free to explore ÔÇö live interactive tech stack map",
  },
};

const jsonLdFAQ = {
  "@context": "https://schema.org",
  "@type": "FAQPage",
  mainEntity: [
    {
      "@type": "Question",
      name: "What is Infinity Tech Stack?",
      acceptedAnswer: {
        "@type": "Answer",
        text: "Infinity is an autonomous self-evolving AI system built by solo founder Bart Chmiel. It features a custom Vitalis compiler (35,632 LOC, 748 tests) and Nova LLM engine written in Rust with Cranelift JIT and CUDA, 72 AI modules, multi-agent swarm consensus, episodic memory, and Asimov's Laws safety governance. 154,000+ total lines of code. The live interactive tech stack map is available at infinitytechstack.uk.",
      },
    },
    {
      "@type": "Question",
      name: "What is the Vitalis programming language?",
      acceptedAnswer: {
        "@type": "Answer",
        text: "Vitalis is a custom compiled programming language purpose-built for autonomous AI code evolution. Written in 35,632 lines of Rust, it compiles through Lexer → Parser → AST → Type Checker → IR → Cranelift JIT → native machine code. It supports @evolvable functions, fitness-based mutation, rollback, and has 748 tests with Python FFI bindings.",
      },
    },
    {
      "@type": "Question",
      name: "How many lines of code is the Infinity AI system?",
      acceptedAnswer: {
        "@type": "Answer",
        text: "Infinity consists of over 154,000 lines of code across three languages: ~98,500 lines of Python (AI backend, 72 modules), ~35,632 lines of Rust (Vitalis compiler + Nova LLM engine with CUDA), and ~19,800 lines of TypeScript (Next.js frontend with cyberpunk dashboard).",
      },
    },
    {
      "@type": "Question",
      name: "Is Infinity open source?",
      acceptedAnswer: {
        "@type": "Answer",
        text: "The Vitalis compiler is open source and available on GitHub at github.com/ModernOps888/vitalis. The full Infinity system architecture is showcased via the interactive tech stack map at infinitytechstack.uk.",
      },
    },
    {
      "@type": "Question",
      name: "Does Infinity offer AI consulting services?",
      acceptedAnswer: {
        "@type": "Answer",
        text: "Yes. Infinity Tech Stack offers enterprise AI consulting including AI Security Audits (┬ú350), AI Architecture Advisory (┬ú400/hr), Custom AI Toolchain Builds, and Microsoft Purview Implementation (┬ú2,500). Contact via b.chmiel20@gmail.com or LinkedIn.",
      },
    },
  ],
};

const jsonLdBreadcrumbs = {
  "@context": "https://schema.org",
  "@type": "BreadcrumbList",
  itemListElement: [
    {
      "@type": "ListItem",
      position: 1,
      name: "Infinity Tech Stack",
      item: "https://infinitytechstack.uk",
    },
    {
      "@type": "ListItem",
      position: 2,
      name: "Tech Stack Map",
      item: "https://infinitytechstack.uk/techstack",
    },
    {
      "@type": "ListItem",
      position: 3,
      name: "AI Consulting",
      item: "https://infinitytechstack.uk/consulting",
    },
  ],
};

const jsonLdPerson = {
  "@context": "https://schema.org",
  "@type": "Person",
  "@id": "https://infinitytechstack.uk/#founder",
  name: "Bart Chmiel",
  url: "https://www.linkedin.com/in/modern-workplace-tech365/",
  jobTitle: "Founder & AI Engineer",
  worksFor: {
    "@id": "https://infinitytechstack.uk/#organization",
  },
  knowsAbout: [
    "Artificial Intelligence",
    "Rust Programming",
    "Compiler Design",
    "Autonomous Systems",
    "AI Safety",
    "FastAPI",
    "Next.js",
    "Microsoft 365",
    "Microsoft Purview",
    "Cloud Architecture",
  ],
  sameAs: [
    "https://www.linkedin.com/in/modern-workplace-tech365/",
    "https://github.com/ModernOps888",
  ],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`dark ${inter.variable} ${jetbrains.variable}`}>
      <head>
        {/* ÔöÇÔöÇ Preconnect hints for performance ÔöÇÔöÇ */}
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="anonymous" />
        <link rel="preconnect" href="https://va.vercel-scripts.com" />

        {/* ÔöÇÔöÇ DNS Prefetch ÔöÇÔöÇ */}
        <link rel="dns-prefetch" href="https://fonts.googleapis.com" />
        <link rel="dns-prefetch" href="https://va.vercel-scripts.com" />
        <link rel="dns-prefetch" href="https://www.google-analytics.com" />
        <link rel="dns-prefetch" href="https://www.googletagmanager.com" />

        {/* ÔöÇÔöÇ JSON-LD Structured Data ÔöÇÔöÇ */}
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdOrganization) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdWebSite) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdSoftwareApplication) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdFAQ) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdBreadcrumbs) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdPerson) }}
        />
      </head>
      <body className={`min-h-screen bg-[var(--bg-primary)] ${inter.className}`}>
        {children}
        <Analytics />
      </body>
    </html>
  );
}
import type { Metadata } from "next";

/* ─────────────────────  CONSULTING PAGE SEO  ───────────────────── */
export const metadata: Metadata = {
  title: "AI Consulting & Security Audits — Infinity Tech Stack",
  description:
    "Enterprise AI consulting from the creator of the Infinity autonomous AI system. AI Security Audits (£350), AI Architecture Advisory (£400/hr), Custom AI Toolchain Builds, and Microsoft Purview Implementation (£2,500). Bart Chmiel — solo-built 154k+ LOC AI system with custom Vitalis compiler (748 tests) and Nova LLM engine.",
  keywords: [
    // ── Services ──
    "AI consulting UK",
    "AI consulting services",
    "AI security audit",
    "AI security assessment",
    "AI architecture consulting",
    "AI architecture advisory",
    "custom AI toolchain",
    "AI toolchain development",
    "Microsoft Purview implementation",
    "Microsoft Purview consulting",
    "Microsoft Purview setup",
    // ── Enterprise ──
    "enterprise AI consulting",
    "enterprise AI security",
    "enterprise AI architecture",
    "AI compliance consulting",
    "AI governance consulting",
    "AI risk assessment",
    // ── Technical ──
    "Rust compiler consulting",
    "FastAPI consulting",
    "AI system design",
    "autonomous AI development",
    "custom programming language development",
    "JIT compiler consulting",
    // ── Location ──
    "AI consulting United Kingdom",
    "AI consultant UK",
    "freelance AI engineer UK",
    "AI developer for hire",
    // ── Brand ──
    "Infinity Tech Stack consulting",
    "Bart Chmiel consulting",
    "Bart Chmiel AI",
  ],
  alternates: { canonical: "/consulting" },
  openGraph: {
    title: "AI Consulting & Security Audits — Infinity Tech Stack",
    description:
      "Enterprise AI consulting from the solo founder who built a 154k+ LOC autonomous AI system. Security audits, architecture advisory, custom toolchains, Microsoft Purview. Proven expertise.",
    url: "https://infinitytechstack.uk/consulting",
    type: "website",
    siteName: "Infinity Tech Stack",
    locale: "en_GB",
    images: [
      {
        url: "/opengraph-image",
        width: 1200,
        height: 630,
        alt: "Infinity Tech Stack — AI Consulting & Security Audits",
      },
    ],
  },
  twitter: {
    card: "summary_large_image",
    title: "AI Consulting & Security Audits — Infinity Tech Stack",
    description:
      "Enterprise AI consulting: Security Audits £350, Architecture Advisory £400/hr, Microsoft Purview £2,500. Built by the creator of a 154k+ LOC autonomous AI system.",
    images: ["/opengraph-image"],
  },
};

/* ── Consulting-specific JSON-LD (injected in head) ── */
const jsonLdService = {
  "@context": "https://schema.org",
  "@type": "ProfessionalService",
  "@id": "https://infinitytechstack.uk/consulting/#service",
  name: "Infinity Tech Stack — AI Consulting",
  url: "https://infinitytechstack.uk/consulting",
  description:
    "Enterprise AI consulting services including AI Security Audits, AI Architecture Advisory, Custom AI Toolchain Builds, and Microsoft Purview Implementation.",
  mainEntityOfPage: {
    "@type": "WebPage",
    "@id": "https://infinitytechstack.uk/consulting",
  },
  provider: {
    "@id": "https://infinitytechstack.uk/#organization",
  },
  areaServed: [
    {
      "@type": "Country",
      name: "United Kingdom",
    },
    {
      "@type": "AdministrativeArea",
      name: "Europe",
    },
  ],
  priceRange: "£350 - £2,500+",
  currenciesAccepted: "GBP",
  paymentAccepted: "Bank Transfer, Invoice",
  knowsLanguage: ["en", "pl"],
  serviceType: [
    "AI Security Audit",
    "AI Architecture Advisory",
    "Custom AI Toolchain Build",
    "Microsoft Purview Implementation",
  ],
  hasOfferCatalog: {
    "@type": "OfferCatalog",
    name: "AI Consulting Services",
    itemListElement: [
      {
        "@type": "Offer",
        itemOffered: {
          "@type": "Service",
          name: "AI Security Audit",
          description:
            "Comprehensive AI security assessment covering model access controls, data pipeline vulnerabilities, prompt injection risks, and compliance gaps.",
        },
        price: "350",
        priceCurrency: "GBP",
        priceSpecification: {
          "@type": "UnitPriceSpecification",
          price: "350",
          priceCurrency: "GBP",
          unitText: "per audit",
        },
      },
      {
        "@type": "Offer",
        itemOffered: {
          "@type": "Service",
          name: "AI Architecture Advisory",
          description:
            "Expert AI architecture review and advisory from a solo founder who designed a 125k+ LOC autonomous AI system with custom compiler, multi-agent swarm, and safety governance.",
        },
        price: "400",
        priceCurrency: "GBP",
        priceSpecification: {
          "@type": "UnitPriceSpecification",
          price: "400",
          priceCurrency: "GBP",
          unitText: "per hour",
        },
      },
      {
        "@type": "Offer",
        itemOffered: {
          "@type": "Service",
          name: "Microsoft Purview Implementation",
          description:
            "End-to-end Microsoft Purview setup for AI governance, information protection, data loss prevention, and compliance management.",
        },
        price: "2500",
        priceCurrency: "GBP",
        priceSpecification: {
          "@type": "UnitPriceSpecification",
          price: "2500",
          priceCurrency: "GBP",
          unitText: "per implementation",
        },
      },
    ],
  },
  email: "b.chmiel20@gmail.com",
  sameAs: [
    "https://www.linkedin.com/in/modern-workplace-tech365/",
  ],
};

const jsonLdConsultingBreadcrumb = {
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
      name: "AI Consulting",
      item: "https://infinitytechstack.uk/consulting",
    },
  ],
};

export default function ConsultingLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdService) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLdConsultingBreadcrumb) }}
      />
      {children}
    </>
  );
}

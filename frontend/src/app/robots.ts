import type { MetadataRoute } from "next";

/**
 * Next.js App Router robots.txt generator
 * Accessible at https://infinitytechstack.uk/robots.txt
 *
 * Allows all crawlers on public pages, blocks dashboard/api/internal routes,
 * and points to the sitemap.
 */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: [
      {
        userAgent: "*",
        allow: ["/", "/techstack", "/consulting"],
        disallow: [
          "/dashboard",
          "/dashboard/*",
          "/api",
          "/api/*",
          "/_next",
          "/_next/*",
        ],
      },
      {
        userAgent: "Googlebot",
        allow: ["/", "/techstack", "/consulting"],
        disallow: ["/dashboard", "/dashboard/*", "/api", "/api/*"],
      },
      {
        userAgent: "Bingbot",
        allow: ["/", "/techstack", "/consulting"],
        disallow: ["/dashboard", "/dashboard/*", "/api", "/api/*"],
      },
    ],
    sitemap: "https://infinitytechstack.uk/sitemap.xml",
    host: "https://infinitytechstack.uk",
  };
}

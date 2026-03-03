import type { MetadataRoute } from "next";

/**
 * Next.js App Router sitemap.xml generator
 * Accessible at https://infinitytechstack.uk/sitemap.xml
 *
 * Keeps Google/Bing crawlers up-to-date with all public pages,
 * priorities, and change frequencies.
 */
export default function sitemap(): MetadataRoute.Sitemap {
  const baseUrl = "https://infinitytechstack.uk";
  const now = new Date();

  return [
    {
      url: baseUrl,
      lastModified: now,
      changeFrequency: "weekly",
      priority: 1.0,
    },
    {
      url: `${baseUrl}/techstack`,
      lastModified: now,
      changeFrequency: "weekly",
      priority: 1.0,
    },
    {
      url: `${baseUrl}/consulting`,
      lastModified: now,
      changeFrequency: "monthly",
      priority: 0.9,
    },
  ];
}

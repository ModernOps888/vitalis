import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

/**
 * Middleware: In production (Vercel), only expose /techstack & /consulting publicly.
 * Locally (localhost), allow all routes for full dashboard access.
 *
 * SEO-critical: Must allow all crawl-essential files through:
 * - /robots.txt, /sitemap.xml, /manifest.webmanifest
 * - /opengraph-image, /icon, /apple-icon
 * - Static assets, Next.js internals
 */
export function middleware(request: NextRequest) {
  const { pathname } = request.nextUrl;
  const host = request.headers.get('host') || '';

  // Allow ALL routes on localhost (dev/local access to full dashboard)
  if (host.startsWith('localhost') || host.startsWith('127.0.0.1')) {
    return NextResponse.next();
  }

  // --- Production: only expose /techstack & /consulting publicly ---

  // Allow root / through — next.config.ts handles 308 permanent redirect to /techstack
  // (308 is stronger SEO signal than middleware's 307)
  if (pathname === '/') {
    return NextResponse.next();
  }

  // Allow techstack page and its sub-paths
  if (pathname === '/techstack' || pathname.startsWith('/techstack/')) {
    return NextResponse.next();
  }

  // Allow consulting page
  if (pathname === '/consulting' || pathname.startsWith('/consulting/')) {
    return NextResponse.next();
  }

  // Allow project deep-dive pages
  if (pathname === '/nova' || pathname.startsWith('/nova/')) {
    return NextResponse.next();
  }
  if (pathname === '/vitalis' || pathname.startsWith('/vitalis/')) {
    return NextResponse.next();
  }

  // Allow Next.js internals, static files, API routes, and SEO-critical files
  if (
    pathname.startsWith('/_next') ||
    pathname.startsWith('/api/public') ||
    // SEO files (crawlers MUST access these)
    pathname === '/robots.txt' ||
    pathname === '/sitemap.xml' ||
    pathname === '/manifest.webmanifest' ||
    // Favicons & OG images
    pathname === '/favicon.ico' ||
    pathname === '/icon' ||
    pathname === '/apple-icon' ||
    pathname === '/opengraph-image' ||
    // Static assets
    pathname.match(/\.(png|jpg|jpeg|gif|svg|ico|webp|avif|woff|woff2|ttf|css|js|json|xml)$/)
  ) {
    return NextResponse.next();
  }

  // Everything else → redirect to /techstack in production
  const url = request.nextUrl.clone();
  url.pathname = '/techstack';
  return NextResponse.redirect(url);
}

export const config = {
  matcher: ['/((?!_next/static|_next/image).*)'],
};

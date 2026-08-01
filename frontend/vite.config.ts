import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { VitePWA } from 'vite-plugin-pwa'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react(),
    // Offline/low-connectivity resilience: this app is meant to be
    // usable in disaster-affected areas, which often means satellite
    // (Starlink or otherwise) or degraded terrestrial links -- higher
    // latency, lower bandwidth, and real drop-outs, not just slowness.
    // A service worker means the app shell (HTML/JS/CSS) and the last
    // successfully fetched live data feed both keep working from cache
    // when the network is gone entirely, rather than showing a blank
    // page. This is what actually makes the app "reachable" on a poor
    // connection -- there's no special integration a satellite ISP
    // needs on the app's side; any standard website works over
    // Starlink or any other ISP the same as over cable/fiber. What
    // matters is that *this* app degrades gracefully when the
    // connection is slow or momentarily absent, which is what this
    // config and src/lib/dataFeed.ts's fetch timeout/retry logic are
    // both for.
    VitePWA({
      registerType: 'autoUpdate',
      workbox: {
        // Precache the built app shell so the UI itself (not just data)
        // loads from cache with zero network round-trips on repeat
        // visits or when fully offline.
        globPatterns: ['**/*.{js,css,html,svg}'],
        runtimeCaching: [
          {
            // The live hazard-data feed (see dataFeed.ts / FR-9):
            // network-first with a short timeout, falling back to the
            // last cached response -- so a stale-but-real last-known
            // state beats both a hang and an empty response when the
            // link is down or too slow.
            urlPattern: /\/data\/communities\.json$/,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'd3rac-live-feed',
              networkTimeoutSeconds: 8,
              cacheableResponse: { statuses: [0, 200] },
              expiration: { maxEntries: 4, maxAgeSeconds: 60 * 60 * 24 * 7 },
            },
          },
          {
            // Google Fonts: cache-first, since typography shouldn't
            // block or fail the page on a slow/absent connection --
            // stale fonts are a non-issue compared to stale hazard data.
            urlPattern: /^https:\/\/fonts\.(googleapis|gstatic)\.com\/.*/,
            handler: 'CacheFirst',
            options: {
              cacheName: 'd3rac-google-fonts',
              expiration: { maxEntries: 20, maxAgeSeconds: 60 * 60 * 24 * 365 },
              cacheableResponse: { statuses: [0, 200] },
            },
          },
        ],
      },
      manifest: {
        name: 'D3R·AC — Data-Driven Disaster Resilience',
        short_name: 'D3R·AC',
        description:
          'Blockchain-powered disaster resilience for communities. Transparent, milestone-based fund disbursement on TRON and Casper.',
        start_url: '/',
        display: 'standalone',
        background_color: '#0b0f14',
        theme_color: '#0b0f14',
        // Only an SVG icon exists in this repo today (favicon.svg) --
        // that's a real gap for installability on platforms that expect
        // sized PNG icons (notably iOS home-screen and some Android
        // launchers); this app is treated as an installable web app for
        // the caching/offline benefit either way, but proper PNG icons
        // (192x192, 512x512, and a maskable variant) are a follow-up
        // item, not something to fabricate here.
        icons: [{ src: '/favicon.svg', sizes: 'any', type: 'image/svg+xml' }],
      },
    }),
  ],
})

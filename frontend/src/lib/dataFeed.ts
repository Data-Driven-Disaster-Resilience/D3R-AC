// FR-9 read-path (frontend side): reads the D3R·AC data pipeline's output
// (data-pipeline/scripts/run_cycle.py writes this to
// frontend/public/data/communities.json every cycle) instead of a
// hardcoded mock array — without ever throwing or leaving the UI blank if
// the feed isn't there yet (e.g. the pipeline hasn't run in this
// environment, or is mid-deployment).
//
// The real long-term read-path is reading RiskRegistry.getCommunity
// directly once Hub/RiskRegistry are deployed (see
// docs/data-pipeline-srs.md's FR-9) — this static-JSON bridge exists
// because that isn't true yet. Swap fetchLiveCommunities' implementation
// for a chain read later without touching useCommunities' call sites.

import { COMMUNITIES, type Community } from "./riskModel";

const FEED_URL = import.meta.env.VITE_D3RAC_FEED_URL ?? "/data/communities.json";

interface PipelineCommunityRow {
  id: string;
  name: string;
  region: string;
  hazard: number;
  exposure: number;
  vulnerability: number;
  lastUpdated: string;
  hazardSource: string;
  stale: boolean;
}

interface PipelineFeed {
  generatedAt: string;
  communities: PipelineCommunityRow[];
}

export type FeedSource = "live" | "demo";

export interface CommunitiesResult {
  communities: Community[];
  source: FeedSource;
  generatedAt: string | null;
  staleCommunityIds: string[];
}

function isValidRow(row: unknown): row is PipelineCommunityRow {
  if (typeof row !== "object" || row === null) return false;
  const r = row as Record<string, unknown>;
  return (
    typeof r.id === "string" &&
    typeof r.name === "string" &&
    typeof r.region === "string" &&
    isUnitInterval(r.hazard) &&
    isUnitInterval(r.exposure) &&
    isUnitInterval(r.vulnerability)
  );
}

function isUnitInterval(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1;
}

function isValidFeed(data: unknown): data is PipelineFeed {
  if (typeof data !== "object" || data === null) return false;
  const d = data as Record<string, unknown>;
  return Array.isArray(d.communities) && d.communities.every(isValidRow);
}

// Milestone data (fundedMilestones/totalMilestones) comes from
// DisbursementController, a different contract the hazard pipeline knows
// nothing about — merged in here from the static demo dataset by id, or
// a neutral 0/1 default if a live community has no local match at all.
function mergeMilestones(row: PipelineCommunityRow): Community {
  const fallback = COMMUNITIES.find((c) => c.id === row.id);
  return {
    id: row.id,
    name: row.name,
    region: row.region,
    hazard: row.hazard,
    exposure: row.exposure,
    vulnerability: row.vulnerability,
    fundedMilestones: fallback?.fundedMilestones ?? 0,
    totalMilestones: fallback?.totalMilestones ?? 1,
  };
}

// Timeout and retry tuning below is deliberately generous relative to
// typical broadband: satellite links (Starlink and other constellations
// alike) commonly add 20-60ms of latency for LEO service, but degrade
// much further on a marginal link (weather fade, obstruction, or a
// distant/overloaded ground station) -- and mobile/rural terrestrial
// connections in the same low-connectivity areas this feed matters most
// for have similar failure modes. Without an explicit timeout, `fetch`
// relies on the browser/OS's own default (often 2+ minutes), which would
// leave the UI showing a loading state far longer than useful, instead
// of falling back to demo data quickly and retrying in the background.
const FETCH_TIMEOUT_MS = 8_000;
const RETRY_DELAYS_MS = [1_000, 3_000]; // one retry, then a second, before giving up for this call

async function fetchWithTimeout(url: string, timeoutMs: number): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { cache: "no-store", signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Fetches the pipeline's live feed. Never throws — any network error,
 * timeout, missing file (404, common if the pipeline hasn't run yet in
 * this environment), or malformed payload resolves to `null` instead, so
 * callers can fall back to demo data without a try/catch of their own.
 *
 * Retries a couple of times with backoff before giving up, since a
 * single dropped packet or momentary satellite-link fade shouldn't be
 * indistinguishable from "no feed exists" -- but each attempt is still
 * bounded by FETCH_TIMEOUT_MS, so a fully dead link still resolves
 * (to the demo-data fallback) in well under the time a hung default
 * fetch would take.
 */
async function fetchLiveCommunities(): Promise<PipelineFeed | null> {
  const attempts = RETRY_DELAYS_MS.length + 1;
  for (let attempt = 0; attempt < attempts; attempt++) {
    try {
      const response = await fetchWithTimeout(FEED_URL, FETCH_TIMEOUT_MS);
      if (!response.ok) return null; // a real 404/5xx isn't worth retrying

      const data: unknown = await response.json();
      if (!isValidFeed(data) || data.communities.length === 0) return null;

      return data;
    } catch {
      // Timeout (AbortError), network error, or JSON parse error. If
      // there's a retry left, back off and try again -- otherwise fall
      // through to the demo-data fallback exactly as before.
      if (attempt < attempts - 1) {
        await delay(RETRY_DELAYS_MS[attempt] ?? 3_000);
        continue;
      }
      return null;
    }
  }
  return null; // unreachable, but keeps the return type honest
}

/**
 * The single entry point the UI should use instead of importing
 * COMMUNITIES directly. Always resolves — never rejects — so a page
 * calling this can render demo data on first paint and never show an
 * error state purely because the live feed isn't reachable.
 */
export async function loadCommunities(): Promise<CommunitiesResult> {
  const feed = await fetchLiveCommunities();

  if (!feed) {
    return {
      communities: COMMUNITIES,
      source: "demo",
      generatedAt: null,
      staleCommunityIds: [],
    };
  }

  return {
    communities: feed.communities.map(mergeMilestones),
    source: "live",
    generatedAt: feed.generatedAt,
    staleCommunityIds: feed.communities.filter((c) => c.stale).map((c) => c.id),
  };
}

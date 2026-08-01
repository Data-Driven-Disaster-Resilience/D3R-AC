import { useEffect, useState } from "react";
import { loadCommunities, type CommunitiesResult } from "./dataFeed";
import { COMMUNITIES } from "./riskModel";

const INITIAL: CommunitiesResult = {
  communities: COMMUNITIES,
  source: "demo",
  generatedAt: null,
  staleCommunityIds: [],
};

/**
 * Loads live pipeline data on mount, replacing the initial demo-data
 * render if/when it resolves. Never enters an error state — dataFeed's
 * loadCommunities() already guarantees a usable result either way, so
 * this hook has exactly two states (loading demo data, then either live
 * or demo data), never a third "failed" state to handle in the UI.
 */
export function useCommunities(): CommunitiesResult & { loading: boolean } {
  const [result, setResult] = useState<CommunitiesResult>(INITIAL);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    loadCommunities().then((r) => {
      if (!cancelled) {
        setResult(r);
        setLoading(false);
      }
    });

    return () => {
      cancelled = true;
    };
  }, []);

  return { ...result, loading };
}

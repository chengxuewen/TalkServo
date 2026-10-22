// One budget, stated once. vite.config's testTimeout (15s) exists because a single AntD
// Table/Drawer mount costs ~900ms of real CPU under jsdom - but Testing Library's own
// per-query budget is separate and defaults to 1000ms, which leaves ~100ms of slack on top
// of a mount. On a warm tree it passes; on a cold one (first run after `npm ci`, no vite
// dep cache, everything contending) a findByText that only waits for an already-queued
// re-render loses that race and reports "Unable to find an element" for code that is fine.
// Hit exactly that on the plan-10 fresh-clone drill (admin-live: rooms_live paints a room the
// poll never returned), reproduced once and then passing 4/4 in isolation - the signature of
// a tight budget, not of a logic bug. Raising the query budget keeps the assertions strict
// (the row still has to arrive from the push, and nothing else ever returns it).
import { configure } from "@testing-library/dom";

configure({ asyncUtilTimeout: 5000 });

// jsdom gaps for AntD
window.matchMedia = window.matchMedia || ((q: string) => ({
  matches: false,
  media: q,
  onchange: null,
  addListener() {},
  removeListener() {},
  addEventListener() {},
  removeEventListener() {},
  dispatchEvent: () => false,
})) as unknown as typeof window.matchMedia;

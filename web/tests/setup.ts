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

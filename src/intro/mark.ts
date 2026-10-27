// VoidTrace mark (the app icon) as a 128x120 cell bitmap for the intro: 0 empty, 1 the V,
// 2 the crimson wedge, 3 the trace line. Rasterised from the polygons in
// src-tauri/icons/source.svg; each row is run-length encoded as <kind><length base 36>.
export const MARK_W = 128;
export const MARK_ASPECT = 1.070; // width / height
const ROWS = ["04.110.0b.2q.0b.110.04.","05.110.0a.2q.0a.110.05.","06.1z.0a.2q.0a.1z.06.","07.1y.0b.2o.0b.1y.07.","07.1z.0a.2o.0a.1z.07.","08.1y.0a.2o.0a.1y.08.","09.1x.0b.2m.0b.1x.09.","09.1y.0a.2m.0a.1y.09.","0a.1x.0a.2m.0a.1x.0a.","0b.1w.0b.2k.0b.1w.0b.","0c.1w.0a.2k.0a.1w.0c.","0c.1w.0b.2i.0b.1w.0c.","0d.1v.0b.2i.0b.1v.0d.","0e.1v.0a.2i.0a.1v.0e.","0e.1v.0b.2g.0b.1v.0e.","0f.1u.0b.2g.0b.1u.0f.","0t.1h.0a.2g.0a.1h.0t.","0t.1h.0b.2e.0b.1h.0t.","0t.1h.010.1h.0t.","0u.1h.0y.1h.0u.","0u.1h.0y.1h.0u.","0u.1h.0y.1h.0u.","0v.1h.0w.1h.0v.","0v.1h.0w.1h.0v.","0w.1h.0u.1h.0w.","0w.1h.0u.1h.0w.","0w.1h.0u.1h.0w.","0x.1h.0s.1h.0x.","0x.1h.0s.1h.0x.","0x.1h.0s.1h.0x.","0y.1h.0q.1h.0y.","0y.1h.0q.1h.0y.","0y.1h.0q.1h.0y.","0z.1h.0o.1h.0z.","0z.1h.0o.1h.0z.","0z.1h.0o.1h.0z.","010.1h.0m.1h.010.","010.1h.0m.1h.010.","010.1h.0m.1h.010.","011.1h.0k.1h.011.","011.1h.0k.1h.011.","011.1h.0k.1h.011.","012.1h.0i.1h.012.","012.1h.0i.1h.012.","012.1h.0i.1h.012.","013.1h.0g.1h.013.","013.1h.0g.1h.013.","013.1h.0g.1h.013.","014.1h.0e.1h.014.","014.1h.0e.1h.014.","014.1h.0e.1h.014.","015.1h.0c.1h.015.","015.1h.0c.1h.015.","015.1h.0c.1h.015.","016.1h.0a.1h.016.","016.1h.0a.1h.016.","016.1h.0a.1h.016.","017.1h.08.1h.017.","017.1h.08.1h.017.","017.1h.08.1h.017.","018.1h.06.1h.018.","018.1h.06.1h.018.","018.1i.04.1i.018.","019.1h.04.1h.019.","019.1h.04.1h.019.","019.1i.02.1i.019.","01a.1h.02.1h.01a.","01a.1h.02.1h.01a.","01a.110.01a.","01b.1y.01b.","01b.1y.01b.","01b.1y.01b.","01c.1w.01c.","01c.1w.01c.","01c.1w.01c.","01d.1u.01d.","01d.1u.01d.","01d.1u.01d.","01e.1s.01e.","01e.1s.01e.","01e.1s.01e.","01f.1q.01f.","01f.1q.01f.","01f.1q.01f.","01g.1o.01g.","01g.1o.01g.","01g.1o.01g.","01h.1m.01h.","01h.1m.01h.","01h.1m.01h.","01i.1k.01i.","01i.1k.01i.","01i.1k.01i.","01j.1i.01j.","01j.1i.01j.","01j.1i.01j.","01k.1g.01k.","01k.1g.01k.","01k.1g.01k.","01l.1e.01l.","01l.1e.01l.","01l.1e.01l.","01m.1c.01m.","01m.1c.01m.","03k.","03k.","03k.","03k.","03k.","03k.","03k.","03k.","03k.","03k.","03k.","03k.","33k.","33k.","33k.","03k."];

export const MARK: Uint8Array[] = ROWS.map((r) => {
  const out = new Uint8Array(MARK_W);
  let x = 0;
  for (const run of r.split(".")) {
    if (!run) continue;
    const n = parseInt(run.slice(1), 36);
    out.fill(Number(run[0]), x, x + n);
    x += n;
  }
  return out;
});

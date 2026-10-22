#!/usr/bin/env python3
"""Regenerates scripts/gate.sh's embedded C1-C5 blocks from
.agents/memorys/conventions.md (mechanism ported from PolyOrch scripts/gate-gen.py).
Run after editing a convention; never hand-edit the embedded regions
(PIT-18 lineage: retyped gate probes lie).

    python3 scripts/gate-gen.py && bash -n scripts/gate.sh
"""
import base64
import pathlib
import re

HERE = pathlib.Path(__file__).parent
conv = (HERE.parent / ".agents/memorys/conventions.md").read_text()
gate = HERE / "gate.sh"
s = gate.read_text()

for n in range(1, 6):
    m = re.search(r"## C%d:[^\n]*\n.*?```bash\n(.*?)```" % n, conv, re.S)
    assert m, "C%d block not found in conventions.md" % n
    b64 = base64.b64encode(m.group(1).rstrip().encode()).decode()
    lines = [b64[i:i + 76] for i in range(0, len(b64), 76)]
    head = ("# ---- C%d (verbatim from conventions.md; do not edit here) ----\n"
            'runb64 "C%d" \\\n') % (n, n)
    pat = re.compile(re.escape(head)
                     + r"(?:.*\n)*?# ---- END C%d ----\n" % n, re.M)
    assert pat.search(s), "C%d region not found in gate.sh" % n
    repl = (
        "# ---- C%d (verbatim from conventions.md; do not edit here) ----\n"
        'runb64 "C%d" \\\n' % (n, n)
        + "\\\n".join('"%s"' % l for l in lines) + "\n"
        "# ---- END C%d ----\n" % n)
    s = pat.sub(lambda _m, r=repl: r, s, count=1)

gate.write_text(s)
print("gate.sh C1-C5 blocks refreshed from conventions.md")

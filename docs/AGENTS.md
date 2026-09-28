# docs — Documentation Tree Knowledge Base

**Parent:** ../AGENTS.md

## OVERVIEW

Three-tier doc system per C2: design (modules/, living) ≠ living reference (reference/, API/config handbooks) ≠ research (reference/research/<domain>/, frozen dossiers).

## STRUCTURE

```
docs/
├── README.md          # index — keep in sync when files move (C2 registry)
├── whitepaper.md      # v0.1 planning voice, D6 annotations inline (do not restyle)
├── architecture.md    # MASTER: thin by design (overview, module index, OQ, acceptance, sister-relation)
├── modules/           # 01-09 design docs: floor-model, signaling-protocol, components, media-pipeline,
│                      # error-model, deployment-security, sdk-strategy, web-ui, dev-toolchain
└── reference/
    ├── README.md      # registry: living refs (empty until crates land) + research index
    └── research/{ptt,media,ui,tooling}/   # 11 frozen dossiers, each with FROZEN banner
```

## WHERE TO LOOK

| Question | Go to |
|----------|-------|
| How does floor arbitration work? | modules/01 + modules/02 sequences J/P/X/R/W |
| What will the repo look like? | modules/03 (crates, features) + docs/README planned table |
| Why mediasoup / Electron / pixi? | architecture lineage + ../../.agents/memorys/decisions.md D6/D10/D11 |
| Market/engine evidence behind choices | research/ dossiers (dates + source URLs in each) |
| New doc goes where? | design→modules/NN; generated schemas/config/ops→reference/; one-time studies→reference/research/<domain>/ new dated file |

## CONVENTIONS (this subtree only)

- Master stays thin: section-level detail belongs in modules/; never restate a module section into the master (drift trap this repo already paid once).
- Frozen dossiers: banner present, conclusions never back-written — supersession is annotated on the LIVE doc citing it.
- Links from `research/<domain>/<x>.md` to docs root use exactly `../../../` (two levels deeper than modules/).
- Citations inside docs follow ledger-first: C/D/PIT ids resolve against ../../.agents/memorys/ or carry "MediaServo".

## ANTI-PATTERNS

- Creating docs/modules/NN-*.md before the corresponding crate/concern exists (P0-1 earned modules/03/09 by DESIGN DECISIONS, not by speculation — keep that bar).
- Counting claims in prose ("16-message contract") — reference the table, counts rot.
- Editing whitepaper.md voice to match modules tone — it is the user-authored planning record with targeted D-notes; revisions go to the revision log.

## COMMANDS

```bash
# run the three gates from repo root: C1 grep, C2 structure, link-integrity (see ../AGENTS.md ## COMMANDS)
find docs -name "*.md" | wc -l    # inventory cross-check vs README tables
```

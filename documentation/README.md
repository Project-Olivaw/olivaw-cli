# olivaw-cli documentation

Deep-dive documentation for contributors and future maintainers. The
authoritative product spec is [CLAUDE.md](../CLAUDE.md) in the repo root;
these documents explain how the implementation satisfies it, the decisions
made along the way, and the lessons that are not obvious from the code.

| document | contents |
| --- | --- |
| [01-project-overview.md](01-project-overview.md) | What olivaw is, the vendoring rationale, the command surface |
| [02-architecture.md](02-architecture.md) | Crate layout, key types, and how a command flows through the code |
| [03-registry-and-components.md](03-registry-and-components.md) | Registry format, component authoring, module-wiring conventions |
| [04-safety-and-drift-detection.md](04-safety-and-drift-detection.md) | Checksums, the three-hash update algorithm, path safety |
| [05-distribution-and-git-registry.md](05-distribution-and-git-registry.md) | Embedded registry, git cache, tag pinning, fallback ladder |
| [06-targets-and-templates.md](06-targets-and-templates.md) | The init scaffolds for esp32, rp2040 and linux |
| [07-testing-and-ci.md](07-testing-and-ci.md) | Test layers, the golden test, CI jobs |
| [08-lessons-learned.md](08-lessons-learned.md) | Non-obvious pitfalls hit during development and their fixes |

Reading order for a new contributor: 01, 02, 03, then whichever area you are
touching. 08 is worth skimming before any change — it records the mistakes
so they are made only once.

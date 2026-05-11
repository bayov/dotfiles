---
axis: dependencies
---

# Dependencies review

Changes to dependency manifests and lockfiles. Reviewed against three questions:
is the new/upgraded dep _necessary_, _safe_, and _correctly pinned_?

## What to look for

### Necessity

- **Net-new dep**: is there a justification visible in the diff (a new feature
  using it, an issue/PR referenced)? Could a small in-repo helper replace it
  (especially for one-function libs)?
- **Trivially-replaceable deps**: any "leftpad-class" dep that wraps a few lines
  of stdlib functionality.
- **Duplication**: does the project already have a dep that solves this problem?
  Two HTTP clients / two date libs / two validation libs in one project is a
  finding.

### Safety

- **Known CVEs**: cross-check the added/upgraded package against publicly known
  vulnerabilities at its pinned version. Where uncertain, flag for the author to
  check the project's vuln scanner.
- **Maintenance status**: is the package actively maintained? Last release
  recency (rough proxy: years-stale = warn). Archived/deprecated upstream =
  finding.
- **Author reputation / supply chain**: new package from an unknown publisher;
  first version; typo-squat-like name close to a well-known package; suspicious
  post-install scripts. These are critical findings — pause and ask the user.
- **License**: license compatible with the project's license? GPL/AGPL pulled
  into a permissive project, or a license absent entirely, is a finding.

### Pinning and versioning

- **Lockfile updated**: the lockfile companion to the manifest is part of the
  same diff. Manifest-only edits are a finding.
- **Pin strictness matches project convention**: if the project uses exact pins,
  a `^x.y.z` is drift; if it uses caret ranges, an exact pin breaks the
  convention.
- **Major-version bumps** call out breaking changes. The diff should reference
  the upstream CHANGELOG or include the migration to the new API.
- **Multiple versions of the same package** transitively resolved (visible in
  lockfile changes): bloat, version-skew risk.

### Footprint

- **Bundle size impact** for frontend changes: a large new dep in a
  client-shipped bundle (cross-check against `performance.md` if perf triggers
  fired too). Prefer narrower / tree-shakeable alternatives.
- **Transitive bloat**: a small surface-area dep pulling in a large dependency
  tree.
- **Native / build-tool deps**: a new dep that requires a system-level build
  tool (gcc, openssl headers) or fails on common platforms. Flag for CI
  compatibility.

### Removals

- A dep removed from the manifest: cross-check the diff to confirm no remaining
  imports reference it.

## What this axis does NOT cover

- The application's own internal modules / build config (covered by
  `conventions.md`).
- Code-level use of the new dep (covered by the relevant code axis once the dep
  is added).

## Severity calibration

- **Critical**: new dep with a known unpatched critical CVE at the pinned
  version; suspected typo-squat / supply-chain signal; license incompatibility.
- **Major**: archived or unmaintained upstream; manifest changed without
  lockfile update; major-version bump with no migration evidence; license
  unclear or missing.
- **Minor**: pin-strictness drift from project convention; transitive
  duplication of a sibling package; one-function dep that could be inlined.

## Output

Per the dispatcher's schema. For findings without a file/line, use `file: null`
and put the dep name and version in the `title` (e.g.
`"left-pad 1.3.0: net-new dep wraps two lines of stdlib"`). `recommendation`
proposes the specific action: bump, pin, replace with X, remove, inline.

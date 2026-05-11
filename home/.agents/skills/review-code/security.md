---
axis: security
---

# Security review

Vulnerabilities and security regressions in the diff. Defensive review; assume
adversarial input on every boundary.

## What to look for

### Injection

- **SQL injection**: any query built by string concatenation or formatting with
  user input. Must use parameterized queries / prepared statements / query
  builders that escape.
- **Command injection**: `exec`, `subprocess`, `child_process`, `os/exec`, shell
  expansion of user-controlled data. `shell=True` with interpolation is a
  finding.
- **Path traversal**: file paths constructed from user input without
  canonicalization and a containment check.
- **Template / SSRF / SSTI**: user input flowing into URL fetchers, template
  renderers, or markup pipelines without explicit allowlists / sanitization.
- **NoSQL/LDAP/XPath injection**: operator-style payloads passed unfiltered to
  query DSLs.
- **HTML / JS injection (XSS)**: user input rendered into HTML without
  context-aware escaping. `dangerouslySetInnerHTML`, `innerHTML`, untrusted
  Markdown rendered as HTML.

### AuthN / AuthZ

- New endpoints / routes / handlers have explicit authentication enforcement; no
  public-by-default.
- Authorization checks happen at the right layer (server, not client only) and
  use the correct subject (the authenticated user, not a request-supplied ID).
- Privilege escalation paths: any endpoint that lets a caller act on a resource
  ID without verifying the caller owns/can-access that ID.
- Multi-tenant isolation: queries scoped to the caller's tenant on every path.
- Session / token handling: tokens not logged, not put in URLs, not stored in
  `localStorage` if they grant privilege, expirations set, rotation possible.

### Secrets and credentials

- No secrets committed to the diff (API keys, passwords, private keys, DB URIs
  with credentials, JWT signing keys, OAuth secrets, certs).
- Secrets read from a secret manager or env, not hard-coded "for testing."
- Generated secrets use cryptographically secure RNGs (`crypto.randomBytes` /
  `secrets` / `crypto/rand`), not `Math.random` / `random.random` / `rand.Int`
  without crypto/rand source.

### Crypto

- No new use of MD5 / SHA-1 for security purposes (hashing passwords, MACs,
  signatures). Use bcrypt/argon2 for passwords; SHA-256+ for integrity; HMAC for
  MACs.
- No homemade crypto. Use vetted libraries.
- TLS verification is on (`verify=True`, no `InsecureSkipVerify`,
  `rejectUnauthorized: true`).
- Constant-time comparisons for secret comparison (`hmac.compare_digest`,
  `subtle.ConstantTimeCompare`, `crypto.timingSafeEqual`).

### Logging and PII

- Secrets, tokens, passwords, full credit-card numbers, SSNs, full emails on
  privacy-sensitive flows — not logged. Headers like `Authorization`, `Cookie`,
  `Set-Cookie` not logged.
- Errors logged with stack/context but with sensitive payloads redacted.

### Configuration and deployment

- New env vars / config keys that grant privilege are documented and have safe
  defaults.
- Dockerfiles / IaC: no `latest` tags pinned in production paths, no privileged
  containers, no permissive IAM (`*` actions on `*` resources), no
  public-by-default storage buckets.

### Dependencies

- New runtime dependencies don't have known CVEs (cross-reference is the
  `dependencies` axis's job; flag here only if the change is _introducing_ the
  dep specifically to bypass a security mechanism).

## Severity calibration

- **Critical**: SQL injection / command injection / auth bypass /
  unauthenticated privileged endpoint / committed secret / cleartext password
  storage / missing TLS verification on a privileged callout.
- **Major**: missing authorization on a non-trivial endpoint; weak crypto for
  security purpose; PII logged; secret read from disk with permissive perms;
  CSRF on state-changing endpoint.
- **Minor**: defense-in-depth missing on a low-risk path; logging that could
  redact more aggressively.

## Output

Per the dispatcher's schema. `recommendation` must say _exactly_ what to do:
which library to swap to, which check to add, what to parameterize. Don't say
"sanitize input" without saying how.

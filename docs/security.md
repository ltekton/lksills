# lskills Security

## Security premise

A skill is model-executed context. Writing a skill into an agent's discovery path
can change behavior even when lskills never executes a file itself. Security gates
therefore run before deployment, and exact source and output hashes remain
verifiable afterward.

lskills governs installation and integrity. It does not sandbox an agent, judge the
semantic safety of instructions, or constrain tools after the agent loads a skill.

## Trust boundaries

### Authored project source

`lskills.toml`, local skills, and forks are operator-controlled source. They may be
edited and versioned normally. Generated target files and internal materializations
are not source.

### External package source

Git repositories, registries, bundles, and local path dependencies are untrusted
input. Source adapters may read and copy them but never execute their content or
package hooks.

### Deployment targets

Agent-native skill directories are active context surfaces. lskills writes only
files present in the authorized composition plan and records ownership and hashes
for every managed output.

## Required controls

### Resolution integrity

- Every Git dependency is locked to an exact commit.
- Every registry or bundle artifact is locked to an archive digest.
- Every package and selected skill tree has a deterministic content digest.
- Normal install never accepts changed bytes under an unchanged lock identity.
- Frozen mode does not select new versions or refs.
- Package name and version metadata are never treated as stronger identity than
  source coordinates and content hashes.

### Authorized file plan

Package validation produces one exact source file inventory after source type, skill
selection, fork replacement, and target selection are known. Explicit aliases then
produce a deterministic output inventory. Security scanning, hashing, copying,
deployment, and packing consume these authorized inventories, and generated alias
content is scanned before deployment.

This prevents an unscanned source-only file from becoming deployable through a
broader copy walk.

### Filesystem safety

- Reject absolute and traversing package-relative paths.
- Reject symlinked path components and archive entries.
- Reject devices, sockets, FIFOs, and unsupported special files.
- Reject non-UTF-8 portable paths rather than rewriting them.
- Preserve bytes and supported executable mode bits as data.
- Validate destination containment before writes and deletes.
- Never invoke Git through a shell; protect user-controlled arguments from option
  injection.

### Content scanning

Before deployment, scan every authorized text file for hidden Unicode classes that
can make instructions visually misleading, including bidi controls, tag characters,
and zero-width content. Critical findings block by default. Any force override is
explicit, narrow, recorded in diagnostics, and does not bypass path, hash, or
archive-integrity checks.

Content scanning does not claim to detect prompt injection, malware, or unsafe
instructions generally.

### Transactionality and concurrency

- Acquire a scope-specific lifecycle lock before the first mutable state read.
- Stage package and target changes on the destination filesystem.
- Validate the complete staged state before activation.
- Atomically replace managed target state where the platform permits.
- Write lockfile and deployment ownership atomically after successful activation.
- Persist enough transaction state to recover or roll back after interruption.
- Read-only commands never recover or mutate state implicitly.

### Ownership-aware deletion

A stale deployed path may be deleted only when:

1. it is within a known target root;
2. the prior lockfile records lskills ownership;
3. no desired skill still claims it;
4. its current bytes match the recorded managed hash.

Edited, shared, malformed, or unowned paths are retained and reported. A stale or
invalid ownership row never authorizes deletion by itself.

### Bundles and publication

A bundle contains an exhaustive sorted path-to-hash manifest. Verification rejects:

- missing listed files;
- extra unlisted files;
- hash mismatches;
- unsafe paths;
- links and special files;
- unsupported schema or artifact type.

Registry versions are immutable. Credentials never appear in manifests, lockfiles,
bundles, diagnostics, or repository URLs. Credential helpers and environment
variables provide host-scoped authentication.

### Fork safety

Forking reads one exact locked skill tree and writes a new local source directory.
It never edits dependency caches or deployed output. The lockfile records the base
revision and digest so later comparisons cannot mistake a different upstream
version for the fork's origin.

An upstream refresh never overwrites a fork. Any future automated rebase must use
the recorded base and surface conflicts as ordinary source changes.

## Audit model

`lskills audit` verifies:

1. manifest parses and agrees with the lockfile;
2. direct and transitive requirements have valid exact resolutions;
3. package, skill, and local-source hashes match;
4. fork bases and local fork hashes are consistent;
5. selected skills and aliases are valid and collision-free;
6. deployment owners reference current package or local identities;
7. deployed paths exist and match recorded hashes;
8. no unmanaged files occupy lskills-owned output paths;
9. the authorized content set passes the security scanner;
10. scratch replay produces the same deployment.

`audit --ci` fails closed if locked content cannot be obtained or replayed. It runs
before any install that could overwrite evidence of drift.

## Explicit non-goals

lskills does not provide:

- runtime sandboxing;
- semantic prompt-injection detection;
- malware analysis of scripts stored as skill resources;
- package signing in the first release;
- publisher identity attestation;
- automatic execution of tests or helper scripts from dependencies;
- hooks, MCP servers, LSP servers, or executable package lifecycle scripts.

Hashes establish byte identity, not publisher trust. Signing and attestations may
be added later without changing the manifest/lock separation.

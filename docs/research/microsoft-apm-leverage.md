# Microsoft APM: concepts, implementation, and leverage for lskills

> Research basis: Microsoft APM v0.33.0 at commit
> [`18c4c43c924ceae890fe0f2038806690e5b2d6c8`](https://github.com/microsoft/apm/tree/18c4c43c924ceae890fe0f2038806690e5b2d6c8).
> Official documentation and implementation were inspected from that fixed
> revision. All APM links below are pinned to it.

## Conclusion

APM and lskills belong to the same product category: package-management systems
for declaratively selecting agent context and reproducibly projecting it into
native runtime locations. APM's most important contribution is the complete chain:

```text
authored manifest
  -> dependency graph and exact lock
  -> verified source materialization
  -> target-selected authorized files
  -> native deployment
  -> ownership, integrity, and replay audit
```

That chain should be the baseline for lskills, not an optional future enhancement.
The previous lskills workarea/import model implemented useful safety mechanisms but
omitted the authored manifest, replay lock, explicit update semantics, project and
global package scopes, dependency graph, and deployment ledger required by the
product goal.

The recommended direction is a separate Rust implementation that is deliberately
compatible with APM's successful package-manager semantics and common reference
forms, while remaining skills-focused and using the requested TOML contracts:

- `lskills.toml` for authored intent;
- `lskills.lock.toml` for exact resolution and deployment state;
- package discovery compatible with common Agent Skills repository shapes;
- concise Git, path, ref, and repository-subpath references;
- install, update, frozen replay, audit, pack, and global/project behavior familiar
  to APM/npm users;
- a first-class fork workflow for editing and republishing acquired skills.

Reusing APM's Python implementation as a library is not a good fit. Its public
schema is YAML, its resolver and installation pipeline cover generic primitives and
executable package capabilities beyond lskills' boundary, and lskills has a Rust
codebase plus a distinct modification workflow. Reimplementing the proven
semantics behind narrow interfaces is lower coupling than embedding or wrapping
the APM CLI. Compatibility should be demonstrated with fixtures, not claimed from
similar terminology.

## 1. What APM establishes

### 1.1 Manifest as concise authored intent

An APM package uses `apm.yml` to declare package metadata, dependencies, package
type, target choices, and optional behavior. Authored primitives live under
`.apm/`, while dependencies materialize under `apm_modules/`. The manifest does not
require users to write resolved commits, package hashes, deployed-file inventories,
or ownership records. Those are generated state.

Dependencies accept familiar compact references: GitHub shorthand, full Git URLs,
optional refs, local paths, and virtual repository paths. This keeps the normal
case comparable to npm rather than forcing every source coordinate into a verbose
record. Structured forms remain available when selection or source detail requires
them.

Sources:

- [Manifest schema](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/reference/manifest-schema.md)
- [Package anatomy](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/concepts/package-anatomy.md)
- [Install packages](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/consumer/install-packages.md)
- [`APMPackage`](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/models/apm_package.py)

For lskills, this establishes the semantic role of `lskills.toml`. TOML is a syntax
choice, not a reason to expose more state to authors. Concise strings remain the
default dependency form, with inline tables only for details such as explicit
skill subsets or aliases.

### 1.2 Lockfile as executable resolved state

`apm.lock.yaml` records enough state to inspect and replay a resolution. Depending
on source and package type, its entries include canonical source identity,
requested and resolved refs, exact commits, versions, dependency ancestry,
package hashes, selected skill or target subsets, deployed paths, per-file hashes,
and deployment ownership.

`apm install --frozen` consumes that record rather than selecting newer upstream
state. Normal install can retain suitable locked resolutions. Update is the
operation that intentionally asks authoritative sources for fresher state. This is
the package-manager distinction between synchronization and freshness.

Sources:

- [Lockfile specification](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/reference/lockfile-spec.md)
- [Dependency management](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/consumer/manage-dependencies.md)
- [Update and refresh](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/consumer/update-and-refresh.mdx)
- [Lockfile implementation](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/deps/lockfile.py)

This is directly applicable. `.lskills/provenance.toml` records useful historical
source facts, but it does not describe a complete graph, selected skills, exact
file inventories, target ownership, or frozen replay. Extending that sidecar would
retain the wrong abstraction. lskills needs a generated lockfile with a specified
canonical TOML encoding.

### 1.3 Resolution is a graph with explanations

APM models direct and transitive dependencies rather than treating the root list as
independent downloads. Its graph retains parent/child relationships, depth,
conflicts, circular dependencies, and flattening for installation. Parent links
matter operationally: a conflict can report both requirement paths instead of only
the final package name.

Sources:

- [Resolver](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/deps/apm_resolver.py)
- [Dependency graph](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/deps/dependency_graph.py)

lskills packages may both export skills and depend on other packages, so this graph
is necessary. Resolution should answer which exact packages exist and why;
composition should separately answer which exported skills are selected, aliased,
replaced by forks, and sent to targets.

### 1.4 Install is a lifecycle pipeline, not a copy command

APM's installation architecture separates source acquisition from shared policy,
integration, cleanup, and lockfile behavior. The pipeline carries distinct source
and destination roots, stages resolution changes, applies security and integrity
checks, integrates target output, reconciles old deployments, and only then
finalizes state.

Important implementation seams include:

- source strategies for local, cached, and newly downloaded dependencies;
- `InstallContext` for explicit source and project roots;
- `ResolutionStagingSession` for reversible replacement;
- a lifecycle lock for cross-process serialization;
- `DeployableSourcePlan` for the exact authorized file set;
- integration and cleanup phases;
- transaction rollback around pipeline failure.

Sources:

- [Install pipeline](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/pipeline.py)
- [Install context](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/context.py)
- [Source strategies](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/sources.py)
- [Resolution staging](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/resolution_staging.py)
- [Lifecycle locking](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/locking.py)
- [Install transaction](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/transaction.py)

lskills should follow this shape while using fewer stages. Its lifecycle is
skills-only and performs no package scripts, prompt execution, or generic primitive
compilation. A complete mutation still needs one plan and transaction spanning
manifest changes, lock state, materialization, target activation, and cleanup.
Atomic per-skill copies are insufficient when an operation changes several skills
and the lockfile together.

### 1.5 One authorized source plan closes a security gap

APM creates a `DeployableSourcePlan` after source, skill subset, target, and
executable authorization are known. Security scanning and deployment consume that
same list. A broad later directory walk therefore cannot deploy an unscanned file.

Sources:

- [Deployable source plan](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/deployable_source_plan.py)
- [Security scan helper](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/helpers/security_scan.py)

This should become a central lskills abstraction. Validation, scanning, hashing,
copying, target deployment, bundle packing, and verbose reporting must consume one
portable, sorted inventory. The current prototype has several good walkers and
digest routines, but they need consolidation around this contract.

### 1.6 Deployment requires ownership, not only file generation

APM records canonical deployment rows. A deployed locator can have several owners,
one active owner, and a content hash. Reconciliation uses these records to retain
shared output, hand off ownership, and remove stale output only when current bytes
still match managed state.

Sources:

- [Deployment ledger](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/core/deployment_ledger.py)
- [Deployment state](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/core/deployment_state.py)
- [Cleanup phase](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/phases/cleanup.py)

lskills needs this once uninstall and update can remove target files. Collision
refusal remains the default composition rule, but it does not answer whether an
old file may be safely deleted. Ownership and recorded hashes do. The lockfile is
the natural canonical record because deployment is generated from a resolution;
a separate opaque provenance database would split the replay contract.

### 1.7 Targets are adapters over resolved content

APM target profiles centralize supported capabilities, native roots, naming,
transforms, and scope support. Integrators consume those profiles instead of
mixing target conditionals into dependency resolution.

Sources:

- [Primitives and targets](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/concepts/primitives-and-targets.md)
- [Target profiles](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/integration/targets.py)
- [Base integrator](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/integration/base_integrator.py)

lskills should use smaller target adapters because it handles only cohesive skill
directories. Adapters decide project/global destination and required native naming
or validation. They must not choose dependency versions or rewrite source identity.
Standard targets should preserve skill bytes unless a documented native contract
requires generated metadata.

### 1.8 Audit means integrity plus replay

APM describes drift across manifest, lockfile, package content, and deployment.
Package and deployed-file hashes provide byte-level evidence, while audit checks
whether the installed context can be explained by current authored and resolved
state.

Sources:

- [Drift and secure by default](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/consumer/drift-and-secure-by-default.md)
- [Content hashing](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/utils/content_hash.py)
- [Integrity policy](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/integrity.py)

For lskills, `audit --ci` should scratch-replay the exact lock and compare logical
deployment output. The prototype's `validate` and generated-output drift checks are
valuable components, but neither proves that dependencies can be reproduced.

## 2. Producer lifecycle and package shapes

APM documents both consumer and producer workflows. Packages can live at a
repository root or virtual subdirectory; a package may contain one skill or expose
multiple skills. Producers validate and preview output before packing. Packed
bundles provide portable distribution rather than requiring consumers to clone an
editable source repository.

Sources:

- [Repository shapes](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/producer/repo-shapes.md)
- [Authoring skills](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/producer/author-primitives/skills.md)
- [Preview and validate](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/producer/preview-and-validate.md)
- [Pack a bundle](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/producer/pack-a-bundle.md)
- [Package types](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/reference/package-types.md)

lskills should preserve the familiar shapes:

```text
one-skill package:             multi-skill package:

lskills.toml                   lskills.toml
SKILL.md                       skills/review/SKILL.md
references/...                 skills/release/SKILL.md
```

Repository-subpath dependencies permit monorepos without inventing a separate
catalog object. Local paths use the same package contract, so a package can be
built and validated before publication.

A package and a packed bundle must remain distinct. A package is editable source
with unresolved intent; a bundle is immutable, self-contained output from one
exact lock state. The bundle needs normalized archive metadata and an exhaustive
path-to-hash manifest so verification can reject both missing and extra content.

## 3. Security lessons and limits

APM treats installed context as security-sensitive even when the package manager
does not execute it. Its `SecurityGate` scans authorized regular files for hidden
Unicode such as bidi controls, tag characters, and zero-width content before
integration. Path checks reject source escapes and links, Git is invoked through
argument APIs, and integrity checks bind source and deployed bytes.

Sources:

- [Security guidance](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/enterprise/security.md)
- [Security gate](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/security/gate.py)
- [Content scanner](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/security/content_scanner.py)

The limits are equally important. These mechanisms do not provide semantic prompt-
injection detection, malware analysis, runtime sandboxing, or proof of publisher
identity. Hashes prove byte identity, not trustworthiness. lskills should state the
same boundary and avoid lifecycle scripts entirely. Scripts and binaries inside a
skill remain copied data and are never run by resolve, install, audit, pack, or
publish.

## 4. Where lskills extends APM

### 4.1 Editable promotion through forks

An installed dependency must remain immutable generated state. Directly editing a
cache or native target makes the change unrepeatable and vulnerable to update or
cleanup. lskills therefore needs a first-class promotion operation:

```text
locked external skill
  -> copy exact tree into local package source
  -> record upstream package, skill, revision, and tree hash
  -> declare local replacement
  -> edit, validate, pack, and publish under a new identity
```

The manifest records the durable relationship, for example:

```toml
[forks.review]
from = "microsoft/team-skills:review"
```

The lockfile records which exact upstream instance formed the base. This supports
three distinct comparisons:

1. local fork versus locked base - what the author changed;
2. locked base versus current upstream - what upstream changed;
3. local fork versus current upstream - the integration delta.

`diff` can expose those views without defining an automatic merge. A future rebase
would need explicit three-way conflict semantics and must never overwrite local
source silently.

### 4.2 Skills-only composition

APM supports several primitive and executable package types. lskills intentionally
narrows the system to Agent Skills. This permits a deeper skill contract:

- each skill is one cohesive directory;
- its directory name is runtime identity;
- all regular descendants are resources;
- package exports and consumer selection are explicit;
- collisions require an alias or fork replacement;
- packing preserves skill boundaries rather than concatenating instructions.

This is not a reduced commitment to reproducibility. It is a smaller asset surface
with the same manifest, resolution, integrity, deployment, and audit rigor.

### 4.3 TOML without semantic divergence

APM uses YAML; lskills is required to use TOML. The useful compatibility target is
semantic:

| Concern | APM reference behavior | lskills adaptation |
|---|---|---|
| Authored file | `apm.yml` | `lskills.toml` |
| Generated file | `apm.lock.yaml` | `lskills.lock.toml` |
| Dependency common case | concise reference string | same familiar reference forms |
| Exact Git state | resolved commit in lock | resolved commit in lock |
| Selection | target/skill subsets | exported skill subset |
| Install | retain suitable locked state | same |
| Update | explicit freshness | same |
| Frozen | no new resolution | same |
| Global use | explicit global scope | independent manifest and lock under `$XDG_CONFIG_HOME/lskills` |
| Deployment | target-native output plus ledger | skills-only target adapters plus ownership rows |

TOML lock output must be canonical: sorted records, stable field ordering, no
volatile timestamps, normalized paths, and no rewrites when semantic state is
unchanged.

## 5. Component-level leverage decision

| Capability | Reuse decision | Reason |
|---|---|---|
| Manifest concepts | Adopt semantics | The intent/resolution split and compact references are foundational. Use TOML and a skills-only schema. |
| APM manifest file compatibility | Adapter later, if demanded | Native lskills is TOML. An APM package reader could import skill exports without making APM YAML the lskills source of truth. |
| Resolver implementation | Reimplement behind a graph interface | APM's Python resolver is coupled to APM models and package types; graph behavior and diagnostics are reusable design evidence. |
| Lockfile model | Adopt semantics, not wire format | Exact source, graph, selection, hash, and deployment state are required; canonical TOML is lskills-specific. |
| Reference grammar | Closely align | Familiar shorthand, refs, local paths, and repository subpaths reduce invention and improve ecosystem fit. |
| Source strategies | Adopt architecture | Local, Git, cache, and later bundle/registry adapters should produce one verified package materialization contract. |
| Installation phases | Adopt and simplify | Plan, stage, scan, integrate, reconcile, and record are needed; generic primitive compilation and scripts are not. |
| Security gate | Reimplement focused subset | Hidden-Unicode and authorized-plan scanning apply directly, but lskills does not inherit executable approval machinery. |
| Deployment ledger | Adopt model | Safe update/uninstall requires owners and hashes. Keep canonical state in the lockfile. |
| Target profiles | Use small concrete adapters | Claude, Codex, and Pi skill roots need shared contracts, not a generic primitive registry. |
| Pack/verify | Adopt reproducibility goals | lskills needs target-neutral deterministic bundles with exhaustive inventory and provenance. |
| Registry/publication | Specify separately before coding | APM proves the lifecycle need; namespace, auth, immutability, and storage protocol are product decisions. |
| Fork workflow | Extend beyond APM baseline | Durable modification and derivative publication are explicit lskills goals. |

The result is neither a wrapper around APM nor an unrelated package manager. It is
a skills specialization that shares user expectations and package-reference
conventions while owning its TOML schema, Rust implementation, modification model,
and artifact protocol.

## 6. Reusable mechanisms in the current lskills prototype

Only after completing the target design was the current code assessed. The useful
parts align well with APM's deeper implementation lessons:

- validated name types and stable error codes;
- `SKILL.md` frontmatter parsing and relative-link validation;
- path containment, symlink rejection, special-file rejection, and portable walks;
- mode-preserving deterministic file enumeration and hashing;
- Git invocation through `Command` arguments and isolated caches;
- staged import with a durable transaction marker and recovery;
- deterministic artifact maps and drift comparison;
- native Claude, Codex, and Pi path knowledge;
- atomic per-skill installation;
- versioned JSON output and actual-binary process tests.

These should be extracted into the new package lifecycle. The following current
concepts should not survive as target interfaces:

- `Repo` as exactly one `skills/` plus `bundles/` aggregate;
- old `Bundle` as a TOML membership list;
- skill identity derived from a bundle-name prefix;
- `import --root` as the primary acquisition workflow;
- `.lskills/provenance.toml` as the only external-source record;
- in-place generated-root publication as package publication;
- a single workarea as the user-facing scope model.

The detailed migration and reuse map is in the
[implementation plan](../implementation-plan.md).

## 7. Risks and decisions still requiring specifications

The product direction is settled, but these details should be decided before their
implementation phase:

1. **Reference grammar edge cases** - exact parsing of host-qualified shorthand,
   fragments, refs, repository subpaths, and transport URLs.
2. **Package identity conflicts** - whether one graph permits multiple versions or
   enforces one resolved identity, and how aliases affect that rule.
3. **Local dependency portability** - what path is recorded in a portable lock and
   how frozen replay behaves after relocation.
4. **Target activation** - atomicity available when native target roots cannot be
   replaced as one directory.
5. **Bundle format** - directory schema, archive normalization, media type, and
   forward compatibility.
6. **Registry protocol** - namespaces, authentication, immutable upload
   finalization, metadata, yanking, and retention.
7. **Fork evolution** - diff is specified; automated rebase remains deferred until
   conflict and base-advance semantics are accepted.

None of these justify falling back to the workarea/import model. They are normal
specification boundaries inside the accepted manifest-and-lock lifecycle.

## 8. Primary-source index

The review covered these official APM areas at the pinned revision:

- top-level direction: `README.md`, `MANIFESTO.md`, `PRINCIPLES.md`, and
  `CONFORMANCE.md`;
- concepts: `what-is-apm.md`, `lifecycle.md`, `package-anatomy.md`,
  `primitives-and-targets.md`, and `the-three-promises.md`;
- consumers: `install-packages.md`, `manage-dependencies.md`,
  `update-and-refresh.mdx`, and `drift-and-secure-by-default.md`;
- producers: skill authoring, repository shapes, package-relative links, preview
  and validation, and bundle packing;
- references: manifest schema, lockfile specification, package types, primitive
  types, and CLI configuration;
- implementation: package model, resolver, dependency graph, lockfile, source
  acquisition, install context and pipeline, staging, transactions, lifecycle
  locking, authorized source planning, integrity, security scanning, integration,
  cleanup, deployment ledger/state, and target profiles.

Pinning the links prevents later upstream changes from silently changing the
research basis.

# ADR-0010 - Manifest-and-lockfile skills package lifecycle

- Status: Accepted
- Date: 2026-10-05
- Supersedes: ADR-0009 as the product and target-architecture decision

lskills will be designed as a skills-focused package manager: `lskills.toml`
declares authored and consumed skills, `lskills.lock.toml` records exact
resolution and deployment state, normal install synchronizes without silent
updates, and project and global scopes follow familiar npm/APM behavior. Internal
workareas, caches, and staging trees are implementation details rather than product
concepts. TOML remains the manifest and lockfile syntax. A first-class fork workflow
extends APM's model by promoting an exact locked external skill into editable local
source with durable upstream provenance, allowing modification, composition,
packing, and republication under a new package identity.

This replaces the earlier one-workarea, one-bundle import model because that model
optimized around the existing prototype rather than the product goal. The prior
implementation may be reused only where it supports the new lifecycle; its command
surface and persisted provenance format are not architectural constraints.

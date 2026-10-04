# lskills Security

> Status: reduced MVP controls

The MVP handles source content as untrusted data and writes only to an explicitly
selected workarea.

## Required controls

### Destination boundary

- `import` requires an explicit `--root`;
- source and destination roots are resolved separately and must be disjoint;
- all bundle, skill, and nested file paths are validated before joins;
- traversal and absolute paths are rejected;
- writes stay inside a staged destination root;
- interrupted staged placements are recovered before another operation reads or
  mutates the workarea;
- an import either completes or leaves no selected bundle/provenance success.

### Source boundary

- local origins are read without mutation;
- Git origins use argument-vector APIs, never a shell;
- source caches remain outside the workarea;
- Git cache entries are keyed by the exact origin and reference and verify their
  configured remote before reuse;
- missing or malformed sources fail closed;
- source hooks, scripts, templates, and binaries are never executed.

### Filesystem content

- unsafe symlinks are rejected and source links are not followed implicitly;
- regular-file and directory kinds are checked throughout the source and workarea
  skills/bundles trees;
- special files are rejected;
- non-UTF-8 relative filenames are rejected rather than rewritten;
- bytes and supported executable modes are preserved;
- hidden and nested files are handled consistently;
- provenance contains locators and revisions, not credentials.

### Publishing

The existing generated roots are renderer-owned output. `publish --check` is the
read-only drift check. Generated roots are validated before drift checks or writes.
Renderer writes reject symlinked parents and non-regular destination files, and do
not follow source symlinks or special files. `publish` does not fetch, execute,
commit, push, or publish to a remote destination.

## Threat boundary

The MVP assumes a trusted local operator and protects against accidental path
escapes, malformed source trees, option injection, and imported content that tries
to execute during lifecycle operations. It does not provide sandboxing for an
operator who later chooses to execute an imported script.

## Deferred safety work

Archive extraction, package hooks, remote publication, multi-workarea credentials,
advanced update safety, and target-specific trust policies are outside this scope.

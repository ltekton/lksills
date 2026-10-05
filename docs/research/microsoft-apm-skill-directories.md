# Microsoft APM skill deployment and version-control behavior

Research basis: `microsoft/apm` at commit [`18c4c43c924ceae890fe0f2038806690e5b2d6c8`](https://github.com/microsoft/apm/tree/18c4c43c924ceae890fe0f2038806690e5b2d6c8), release v0.33.0. This note answers whether agent-native skill deployment directories are expected to be committed or ignored, and how APM makes either workflow safe.

## Answer

APM's default and documented recommendation is to **commit agent-native deployment outputs**, including `.agents/skills/` and target-specific directories such as `.claude/`. It automatically adds only `apm_modules/` to the project's committed `.gitignore`. It does not automatically add generated skill deployment directories to `.gitignore` or `.git/info/exclude`.

APM also supports repositories that deliberately gitignore deployed output, but treats that as an alternate repository policy. Such repositories must run `apm install` to materialize skills after a fresh clone. APM's audit implementation recognizes gitignored missing deployment paths so that their deliberate absence does not fail the `deployed-files-present` check.

## Source and deployment directories are separate

APM distinguishes editable source from runtime deployment:

```text
.apm/skills/<name>/...       # authored package source
.agents/skills/<name>/...    # generated cross-client deployment
.claude/skills/<name>/...    # generated Claude deployment
.kiro/skills/<name>/...      # generated Kiro deployment
```

Its package-anatomy documentation calls `.apm/` the source tree and target-native directories generated output. It tells authors to edit `.apm/` and rerun install rather than editing a deployed copy ([package anatomy](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/concepts/package-anatomy.md)).

Most supported clients converge on `.agents/skills/`. The target definitions route Copilot, Cursor, OpenCode, Gemini, Codex, and Windsurf skills through `deploy_root=".agents"`; Claude retains `.claude/skills/` ([target profiles](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/integration/targets.py)). The migration guide documents this convergence and says migrated `.agents/skills/` output can be committed with the updated lockfile ([skill path migration](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/getting-started/migration.md)).

## APM's commit policy

The quickstart's "What to commit" table says to commit:

- `apm.yml`;
- `apm.lock.yaml`;
- target-owned deployment directories, including `.agents/`, `.github/`, `.claude/`, and `.grok/`.

It says not to commit `apm_modules/`, which is restored from the lockfile by `apm install` ([quickstart](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/quickstart.mdx)). The consumer installation guide repeats the same rule and explains the rationale: committed agent files are reviewable and discoverable immediately after clone, although linked package context can still require `apm install` ([install packages](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/consumer/install-packages.md)).

The implementation matches the documentation. During project installation, the pipeline calls `_update_gitignore_for_apm_modules`; that helper appends only:

```gitignore
# APM dependencies
apm_modules/
```

The helper has no target-output rules, and global installs skip this update ([pipeline](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/install/pipeline.py); [Git-ignore helper](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/commands/_helpers.py)). A source search at the pinned revision found no APM logic that manages `.git/info/exclude`.

APM dogfoods this policy. At the pinned revision, the APM repository's `.gitignore` ignores `apm_modules/` but not `.agents/skills/`; the repository tracks both `.apm/skills/` source and `.agents/skills/` deployment output. Its checked-in lockfile records those deployed paths and their hashes ([repository `.gitignore`](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/.gitignore); [source skills](https://github.com/microsoft/apm/tree/18c4c43c924ceae890fe0f2038806690e5b2d6c8/.apm/skills); [deployed skills](https://github.com/microsoft/apm/tree/18c4c43c924ceae890fe0f2038806690e5b2d6c8/.agents/skills); [lockfile](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/apm.lock.yaml)).

## Gitignored deployments are supported, not preferred

APM explicitly accommodates repositories that choose not to commit deployment output:

- `deployed-files-present` collects missing paths from the lockfile, runs `git check-ignore --stdin -z`, and excludes ignored paths from the failure set. If Git is unavailable or the check fails, APM reports the missing paths rather than silently passing ([CI check implementation](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/policy/ci_checks.py)).
- Integration tests cover both sides: an absent `.agents/skills/...` path passes when `.agents/` is ignored, while an absent non-ignored deployment fails ([Git-ignore integration tests](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/tests/integration/test_deployed_files_gitignore_integration.py)).
- CI documentation distinguishes committed-output repositories from gitignored-output repositories. Committed output supports audit-only CI that checks checked-out bytes without first overwriting them. Gitignored output normally uses install-then-audit so the runtime files exist on a fresh runner ([CI enforcement](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/enterprise/enforce-in-ci.md); [drift detection](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/enterprise/drift-detection.md)).

APM therefore does not force one Git policy. It recommends committed outputs, leaves ignore policy to the repository, and makes audit aware of the repository's decision.

## Ownership and safe reconciliation

Committed output is not treated as uncontrolled generated clutter. APM's portable lockfile records:

- each deployed path;
- its content hash;
- its owner or owners;
- the active owner when multiple packages claim a destination;
- local-source deployments as well as dependency deployments.

That data supports fresh-clone integrity checks and replay without relying on an uncommitted machine receipt ([lockfile example and field discussion](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/concepts/package-anatomy.md); [deployment ledger codec](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/core/deployment_ledger.py)).

For collisions, a path previously tracked as APM-managed can be overwritten on reinstall. An untracked pre-existing path is preserved with a warning unless the user explicitly supplies `--force`. During stale-file cleanup, a changed file whose current hash no longer matches APM's recorded hash is preserved as a user edit rather than deleted ([security model](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/docs/src/content/docs/enterprise/security.md); [cleanup implementation](https://github.com/microsoft/apm/blob/18c4c43c924ceae890fe0f2038806690e5b2d6c8/src/apm_cli/integration/cleanup.py)).

This lockfile-backed ownership is central to APM's committed-output model. Merely committing generated files without portable expected-path and expected-hash information would lose much of its audit and cleanup safety.

## Fresh-clone behavior

The two supported policies have different semantics:

| Repository policy | Immediately agent-discoverable after clone | Requires install before use | Can review deployed bytes in a pull request | Typical CI pattern |
|---|---:|---:|---:|---|
| Commit target output | Yes | Sometimes, for linked package context | Yes | Audit-only against lock-pinned scratch replay |
| Ignore target output | No | Yes | No | Install, then audit |

APM chooses the first as its recommendation because agent-visible files are treated as reviewable project deliverables. `apm_modules/` remains disposable dependency materialization and is ignored.

## Implications for lskills

The APM evidence changes the earlier lskills recommendation in one important way: automatically adding every generated skill path to `.git/info/exclude` would not follow APM's default. It would silently select APM's supported but secondary gitignored-output workflow.

Lskills has three coherent choices:

1. **Follow APM's default.** Commit native deployment output. Keep downloaded package objects outside the repository. Record a portable projection inventory and hashes in `.lskills/lock.toml`; keep machine-only mutation receipts in the state directory.
2. **Choose package-manager-style materialization.** Ignore native deployment output and require `lskills install --frozen` after clone. Make audit distinguish deliberately ignored absence from unexpected missing files, as APM does.
3. **Make the policy explicit per project.** Default to one mode, but allow a manifest setting to select committed or local-only deployment. CI and audit behavior must follow that declared mode rather than infer policy only from incidental Git-ignore rules.

The first option provides immediate agent discoverability and reviewable prompt content but duplicates source/materialized files and creates generated diffs. The second keeps the repository minimal but makes installation a prerequisite for agent behavior. The third supports both workflows but adds a product-level mode and testing matrix.

Regardless of the choice, lskills should not silently manage `.git/info/exclude` before deciding the repository contract. APM demonstrates that Git treatment, fresh-clone behavior, lockfile deployment inventory, drift checks, and CI workflow are one coupled design decision.

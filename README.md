# ship

**One command — ship.**

A fast pre-deploy checklist CLI written in Rust.

```
$ ship

ship

Checks
  ✓ tests
  ✓ secrets
  ✓ TODOs
  ✓ console.logs
  ✓ feature flags
  ✓ version
  ✓ migrations
  ✓ changelog

✓ Ready to ship
```

## Checks

| Check            | Critical | Description |
|------------------|----------|-------------|
| **tests**        | yes      | Runs the project test suite (`cargo test`, `npm test`, `pytest`, `go test`, …) |
| **secrets**      | yes      | Scans source for private keys, API tokens, connection strings, high-entropy assignments |
| **TODOs**        | no       | Finds `TODO` / `FIXME` / `XXX` / `HACK` comments |
| **console.logs** | no       | Finds leftover debug prints (`console.log`, `dbg!`, `print(`, …) |
| **feature flags**| no       | Detects feature-flag usage and suspicious temporary flags |
| **version**      | no       | Reads version from `Cargo.toml` / `package.json` / `pyproject.toml` / git tags |
| **migrations**   | no       | Detects migration folders / tools (Prisma, Diesel, Django, Alembic, …) |
| **changelog**    | no       | Checks for `CHANGELOG.md` (and whether it looks updated since last tag) |

Critical failures (tests, secrets) cause a non-zero exit code so you can gate deploys.

For a slower, more thorough pass — expanded secret patterns, tracked `.env` files, and more — see [`ship security`](#deep-security-scan-ship-security) below.


## Install

### From source

```bash
cargo install --path .
```

### Pre-built binaries — Linux & macOS

Install from a GitHub release binary (recommended):

**Linux x86_64:**
```bash
curl -sSL https://github.com/dominicOT/ship/releases/latest/download/ship-linux-x86_64.tar.gz | tar xz -C ~/.local/bin ship
```

**Linux ARM64:**
```bash
curl -sSL https://github.com/dominicOT/ship/releases/latest/download/ship-linux-aarch64.tar.gz | tar xz -C ~/.local/bin ship
```

**macOS (Intel):**
```bash
curl -sSL https://github.com/dominicOT/ship/releases/latest/download/ship-macos-x86_64.tar.gz | tar xz -C /usr/local/bin ship
```

**macOS (Apple Silicon):**
```bash
curl -sSL https://github.com/dominicOT/ship/releases/latest/download/ship-macos-aarch64.tar.gz | tar xz -C /usr/local/bin ship
```

Or use the one-line installer:

```bash
curl -sSL https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.sh | bash
```

To install to a custom directory:

```bash
curl -sSL https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.sh | bash -s -- --prefix ~/.local/bin
```

### Pre-built binaries — Windows

Run this in PowerShell (or PowerShell Core):

```powershell
irm https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.ps1 | iex
```

Or if you get an execution policy error:

```powershell
powershell -ExecutionPolicy Bypass -Command "& { irm https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.ps1 | iex }"
```

**Windows x86_64 downloads directly from:**
```
https://github.com/dominicOT/ship/releases/latest/download/ship-windows-x86_64.zip
```

Unzip and place `ship.exe` somewhere in your PATH, or run the PowerShell installer above to handle it automatically.

### Supported platforms

| Platform | Architecture | Status |
|---|---|---|
| Linux | x86_64 | ✓ |
| Linux | ARM64 | ✓ |
| macOS | Intel (x86_64) | ✓ |
| macOS | Apple Silicon (ARM64) | ✓ |
| Windows | x86_64 | ✓ |

For other platforms or architectures, install from source with `cargo install --path .`

## Usage

```bash
ship                            # run all checks in current directory
ship --project /path/to/project # run checks on a specific project directory (or -p)
ship -v                         # verbose (show details / snippets)
ship -n                         # dry-run (never fail the process)
ship --skip todos,logs          # skip specific checks
ship --only secrets,tests
```

## Exporting reports

You can export the report as JSON or Markdown for later inspection or to feed into an agent:

```bash
ship --json            # writes ship-report.json by default
ship --md              # writes ship-report.md by default
ship --json out.json   # write JSON to a specific path
ship --md report.md    # write Markdown to a specific path
ship --json --md       # write both default files
```

When `--json` or `--md` are provided without a path, the CLI defaults to `ship-report.json` and `ship-report.md` respectively.

## Deep security scan (`ship security`)

`ship` itself stays fast — the default `secrets` check is deliberately lightweight so it's safe to run on every commit. For a slower, more thorough pass, run:

```bash
ship security                     # run all deep security checks
ship security -p /path/to/project # target a specific project directory (or --project)
ship security -v                  # verbose (show details / snippets)
ship security -n                  # dry-run (never fail the process)
ship security --skip env-files,deps  # skip specific checks
ship security --only secrets
ship security --json              # writes ship-security-report.json by default
ship security --md                # writes ship-security-report.md by default
```

| Check         | Description |
|---------------|-------------|
| **secrets**   | Expanded pattern set beyond the default scan: JWT tokens and signing secrets, private keys (RSA/EC/OpenSSH/DSA/PGP), GCP service account key files, cloud provider tokens (AWS, Azure, DigitalOcean, Docker Hub, npm, PyPI), vendor API keys (GitHub, OpenAI/Anthropic, Stripe, Slack, Twilio, SendGrid, Mailgun), and connection strings. Also scans infra/CI files that the fast check skips by extension: Terraform (`.tf`/`.tfvars`), Dockerfiles, `docker-compose.yml`, Jenkinsfiles. |
| **env-files** | Flags any `.env` / `.env.*` file that's tracked by git — i.e. already committed, not just present on disk. Templates (`.env.example`, `.env.sample`, `.env.template`, …) are excluded. |
| **deps**      | Parses `Cargo.lock` / `package-lock.json` / `poetry.lock` / `go.sum` and flags: (1) resolved versions matching a small curated advisory list, (2) missing lockfiles (skipped for Rust libraries, where an unpinned `Cargo.lock` is normal), (3) `*` / unbounded version ranges (`>=x` with no upper bound) on security-sensitive direct dependencies (auth, crypto, serialization, web frameworks, …), and (4) unpinned entries in `requirements.txt`. |
| **auth**      | Framework-aware auth/JWT smells in JS/TS and Python source: the JWT `"none"` algorithm, a signing secret passed directly as a string literal to `jwt.sign`/`verify`/`encode`, a `jwt.verify`/`decode` call with no explicit `algorithms` allowlist (the root cause behind several real algorithm-confusion CVEs), Express routes on a sensitive path (`/admin`, `/account`, `/payment`, …) with no middleware between the path and the handler, and Flask `@app.route` views on a sensitive path with no `login_required`/`jwt_required`-style decorator nearby. |

The `deps` advisory list is a small, hand-picked seed (a handful of well-known npm/PyPI/crates.io CVEs) — not a replacement for `cargo audit`, `npm audit`, or `pip-audit`, which pull from full vulnerability databases. Likewise, `auth`'s missing-middleware/missing-decorator checks are regex heuristics, not real parsing — they're tuned to avoid false positives (they only fire on sensitive-looking paths, and only when middleware/decorators are genuinely absent nearby), but they can still miss auth enforced further up a call chain.

More checks are planned: Django/Rails auth coverage, and broader framework support beyond Express/Flask.

## Git Hook Setup (`ship init`)

Set up Ship as an automatic, committed git `pre-commit` hook for your project:

```bash
ship init
# or for a specific project:
ship init --project /path/to/project
```

This:
1. Creates `.githooks/pre-commit` (making it executable).
2. Configures git to use the shared hooks directory: `git config core.hooksPath .githooks`.
3. Creates a `.ship.toml` configuration file (if not already present).

By default the hook skips `tests` (usually too slow for a pre-commit hook) and runs everything else. Pick your own checklist for the hook:

```bash
ship init --skip tests,logs      # skip the tests and console.log checks
ship init --only secrets,todos   # only run these checks, ignore the rest
ship init --all                  # run every check, including tests
```

Re-run `ship init` any time to change the selection later — it overwrites the existing hook.

Commit the hooks so your entire team gets them:
```bash
git add .githooks .ship.toml
git commit -m "chore: add ship pre-commit hook"
```

Teammates who clone the repository just need to enable the hooks once:
```bash
git config core.hooksPath .githooks
```

## Updating (`ship update`)

If you installed ship from a pre-built binary, keep it up to date in place:

```bash
ship update          # download and install the latest release
ship update --check  # only check whether an update is available
ship update --force  # reinstall the latest release even if already up to date
```

Requires `curl` (and `tar` on Linux/macOS, `unzip` on Windows). If you installed via `cargo install --path .`, reinstall from source instead.

## Supported project types

- **Rust** (`Cargo.toml`)
- **Node.js** (`package.json` + npm / yarn / pnpm)
- **Python** (`pyproject.toml` / `setup.py` / `requirements.txt`)
- **Go** (`go.mod`)

Unknown projects still get secrets / TODO / changelog scans.

## Philosophy

Ship is intentionally small and fast. It is meant to be the last local gate before `git push` or a deploy script — not a replacement for CI.

The default checks stay fast enough for a pre-commit hook. Anything slower or broader — more secret patterns, dependency/CVE checks, framework-aware smells — lives behind `ship security` instead of creeping into the default path.

```
ship && ./deploy.sh
```

## License

MIT

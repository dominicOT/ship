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


## Supported project types

- **Rust** (`Cargo.toml`)
- **Node.js** (`package.json` + npm / yarn / pnpm)
- **Python** (`pyproject.toml` / `setup.py` / `requirements.txt`)
- **Go** (`go.mod`)

Unknown projects still get secrets / TODO / changelog scans.

## Philosophy

Ship is intentionally small and fast. It is meant to be the last local gate before `git push` or a deploy script — not a replacement for CI.

```
ship && ./deploy.sh
```

## License

MIT

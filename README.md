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

Install from source:

```bash
cargo install --path .
```

Install a locally built binary:

```bash
cargo build --release
cp target/release/ship ~/.local/bin/
```

Install from a GitHub release binary:

```bash
curl -sSL https://github.com/dominicOT/ship/releases/latest/download/ship-linux-x86_64.tar.gz | tar xz -C ~/.local/bin ship
```

Install with the one-line shell installer:

```bash
curl -sSL https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.sh | bash
```

To install to a custom directory:

```bash
curl -sSL https://raw.githubusercontent.com/dominicOT/ship/master/scripts/install.sh | bash -s -- --prefix ~/.local/bin
```

## Usage

```bash
ship                    # run all checks
ship -v                 # verbose (show details / snippets)
ship -n                 # dry-run (never fail the process)
ship --skip todos,logs  # skip specific checks
ship --only secrets,tests
```

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

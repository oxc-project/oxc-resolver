#!/usr/bin/env -S just --justfile

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set shell := ["bash", "-cu"]

_default:
  @just --list -u

alias r := ready

# Make sure you have cargo-binstall installed.
# You can download the pre-compiled binary from <https://github.com/cargo-bins/cargo-binstall#installation>
# or install via `cargo install cargo-binstall`
# Initialize the project by installing all the necessary tools.
init:
  cargo binstall cargo-shear typos-cli watchexec-cli -y
  rustup target add s390x-unknown-linux-gnu

install:
  pnpm install
  cd fixtures/pnp; yarn
  cd fixtures/pnp/global-pnp; yarn
  cd fixtures/yarn; yarn

# When ready, run the same CI commands
ready:
  git diff --exit-code --quiet
  just install
  typos
  cargo fmt
  just check
  just test
  just lint
  just doc
  git status

watch *args='':
  watchexec {{args}}

watch-check:
  just watch "'cargo check; cargo clippy'"

watch-example target *args='':
  just watch "cargo run --example {{target}} -- {{args}}"

# Run the example
example target *args='':
  cargo run --example {{target}} -- {{args}}

# Format all files
fmt:
  cargo shear --fix # remove all unused dependencies
  cargo fmt --all
  node --run fmt

# Run cargo check
check:
  cargo check --all-features --all-targets
  cargo check --target s390x-unknown-linux-gnu

# Run all the tests
test:
  cargo test
  cargo test --all-features
  node --run build
  node --run test
  cd fixtures/pnp; yarn test

# Lint the whole project
lint:
  cargo clippy --all-features --all-targets -- --deny warnings

# Generate doc
doc:
  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features

# Get code coverage
codecov:
  cargo codecov --html

# Run the benchmarks.
benchmark:
  cargo bench --bench resolver

# Install one nested-monorepo fixture per pm * mode under fixtures/bench-pm/. Heavy; not part of `just install`.
install-bench-fixtures:
  cd fixtures/bench-pm/npm-flat       && npm install --no-audit --no-fund
  cd fixtures/bench-pm/pnpm-isolated  && pnpm install
  cd fixtures/bench-pm/pnpm-hoisted   && pnpm install
  cd fixtures/bench-pm/yarn-nm        && yarn install
  cd fixtures/bench-pm/yarn-pnpm      && yarn install
  cd fixtures/bench-pm/yarn-pnp       && yarn install
  command -v bun >/dev/null && (cd fixtures/bench-pm/bun-flat && bun install) || echo 'skip bun-flat: bun not installed'
  command -v bun >/dev/null && (cd fixtures/bench-pm/bun-isolated && bun install) || echo 'skip bun-isolated: bun not installed'

# Run the package-manager benchmarks. Each combo skips itself if its fixture is not installed.
benchmark-pm:
  cargo bench --bench package_managers --features yarn_pnp

# Run cargo-fuzz
fuzz:
  cd fuzz; cargo +nightly fuzz run --sanitizer none resolver -- -only_ascii=1 -max_total_time=900

# Manual Release
release:
  cargo binstall -y release-plz
  release-plz update
  just check
  # NOTE: make sure to update version in npm/package.json

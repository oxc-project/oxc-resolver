#!/usr/bin/env -S just --justfile

_default:
  @just --list -u

alias r := ready

# Make sure you have cargo-binstall installed.
# You can download the pre-compiled binary from <https://github.com/cargo-bins/cargo-binstall#installation>
# or install via `cargo install cargo-binstall`
# Initialize the project by installing all the necessary tools.
init:
  cargo binstall cargo-shear dprint typos-cli watchexec-cli -y

install:
  pnpm install
  cd fixtures/pnp && yarn
  cd fixtures/global-pnp && yarn

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

watch-example *args='':
  just watch "cargo run --example resolver -- {{args}}"

# Run the example
example *args='':
  cargo run --example resolver -- {{args}}

# Format all files
fmt:
  cargo shear --fix # remove all unused dependencies
  cargo fmt --all
  dprint fmt

# Run cargo check
check:
  cargo check --all-features --all-targets

# Run all the tests
test:
  cargo test --all-features
  node --run build
  node --run test

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
  cargo bench

# Run cargo-fuzz
fuzz:
  cd fuzz && cargo +nightly fuzz run --sanitizer none resolver -- -only_ascii=1 -max_total_time=900

# Manual Release
release:
  cargo binstall -y release-plz
  release-plz update
  just check
  # NOTE: make sure to update version in npm/package.json

#!/usr/bin/env -S just --justfile

_default:
  @just --list -u

alias r := ready

# Make sure you have cargo-binstall installed.
# You can download the pre-compiled binary from <https://github.com/cargo-bins/cargo-binstall#installation>
# or install via `cargo install cargo-binstall`
# Initialize the project by installing all the necessary tools.
init:
  cd fixtures/pnpm8 && pnpm install
  cargo binstall cargo-watch typos-cli taplo-cli cargo-llvm-cov -y

# When ready, run the same CI commands
ready:
  git diff --exit-code --quiet
  typos
  cargo fmt
  just check
  just test
  just lint
  git status

# --no-vcs-ignores: cargo-watch has a bug loading all .gitignores, including the ones listed in .gitignore
# use .ignore file getting the ignore list
# Run `cargo watch`
watch command:
  cargo watch -x '{{command}}'

# Run the example in `parser`, `formatter`, `linter`
example *args='':
  just watch 'run --example resolver -- {{args}}'

# Format all files
fmt:
  cargo fmt
  taplo format

# Run cargo check
check:
  cargo check

# Run all the tests
test:
  cargo test

# Lint the whole project
lint:
  cargo clippy -- --deny warnings

# Generate doc
doc:
  RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features

# Get code coverage
codecov:
  cargo codecov --html

# Run the benchmarks. See `tasks/benchmark`
benchmark:
  cargo benchmark

# Run cargo-fuzz
fuzz:
  cd fuzz && cargo +nightly fuzz run --sanitizer none resolver -- -only_ascii=1 -max_total_time=900

release:
  release-plz update

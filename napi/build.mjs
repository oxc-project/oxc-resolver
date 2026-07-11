// Wrapper around `napi build` (the `build:debug` script) so release builds can
// inject cargo configuration the napi CLI does not expose as flags.
import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { createBuildCommand, NapiCli } from "@napi-rs/cli";

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// The same clipanion parser as the `napi build` CLI, so flags like `-x` and
// `--use-napi-cross` behave identically.
const buildCommand = createBuildCommand(process.argv.slice(2));
const argsOptions = buildCommand.getOptions();
// `getOptions()` does not surface args after `--`; the CLI's BuildCommand forwards those
// separately as `cargoOptions`, so read the same field to keep the passthrough working.
const restCargoOptions = buildCommand.cargoOptions ?? [];

const isRelease = argsOptions.release === true || argsOptions.profile === "release";

// For published binaries, remap the absolute build-machine paths (cargo/rustup homes and
// the workspace root) that rustc embeds into panic locations and tracing callsite metadata.
// This shrinks the binary's string tables and keeps the build machine's filesystem layout
// out of the shipped artifact. Release-only so local dev backtraces keep clickable paths.
// Replace with cargo `-Ztrim-paths` once it stabilizes (rust-lang/cargo#12137).
//
// The flags are injected as a cargo `--config target.'cfg(all())'.rustflags=[…]` entry, NOT
// via RUSTFLAGS/CARGO_BUILD_RUSTFLAGS: config-level target rustflags are joined with the
// `.cargo/config.toml` target entries (the gnu `nodelete` link-args, the wasi stack size),
// whereas the napi CLI promotes CARGO_BUILD_RUSTFLAGS to RUSTFLAGS, which suppresses all
// config-level target rustflags.
// Known gap: napi-cli always sets RUSTFLAGS for musl targets (`-C target-feature=-crt-static`),
// which suppresses config-level rustflags there — musl artifacts keep unremapped paths. The
// durable fix is upstream in napi-rs.
let remapConfig;
if (isRelease) {
  const cargoHome = process.env.CARGO_HOME ?? resolve(homedir(), ".cargo");
  const rustupHome = process.env.RUSTUP_HOME ?? resolve(homedir(), ".rustup");
  const remaps = [
    `--remap-path-prefix=${cargoHome}=/cargo`,
    `--remap-path-prefix=${rustupHome}=/rustup`,
    `--remap-path-prefix=${workspaceRoot}=/oxc-resolver`,
  ];
  // Collapse the long per-registry hash directory (`registry/src/index.crates.io-<hash>`)
  // too: rustc uses the last matching prefix, so these more-specific mappings go last.
  // The registry extraction dirs only exist after dependencies are fetched, and this script
  // runs before napi invokes cargo — on a cold CI runner the directory would be empty. Fetch
  // first (cheap: the same download cargo would do anyway), then enumerate sorted so the
  // resulting flag set is deterministic.
  spawnSync(
    "cargo",
    ["fetch", "--locked", ...(argsOptions.target ? ["--target", argsOptions.target] : [])],
    { cwd: workspaceRoot, stdio: "inherit" },
  );
  const registrySrc = resolve(cargoHome, "registry", "src");
  try {
    for (const dir of readdirSync(registrySrc).sort()) {
      remaps.push(`--remap-path-prefix=${resolve(registrySrc, dir)}=/deps`);
    }
  } catch {
    // no registry dir (e.g. vendored deps) — nothing to collapse
  }
  // `-Z build-std` compiles std from the sysroot's rust-src, so the `/rustup` mapping
  // alone still leaves `toolchains/<tc>/lib/rustlib/src/rust/…` in panic locations.
  // Collapse that tree too: `…/library/alloc/…` → `/std/alloc/…`, and std's vendored
  // dependencies → `/deps/<crate>` to match the registry mapping.
  const sysroot = spawnSync("rustc", ["--print", "sysroot"], { encoding: "utf8" });
  if (sysroot.status === 0) {
    const rustSrc = resolve(sysroot.stdout.trim(), "lib", "rustlib", "src", "rust");
    remaps.push(
      `--remap-path-prefix=${resolve(rustSrc, "vendor")}=/deps`,
      `--remap-path-prefix=${resolve(rustSrc, "library")}=/std`,
    );
  }
  // TOML literal strings cannot contain single quotes; such paths just skip the remap.
  if (remaps.every((flag) => !flag.includes("'"))) {
    remapConfig = `target.'cfg(all())'.rustflags=[${remaps.map((flag) => `'${flag}'`).join(",")}]`;
  }
}

// Rebuild std without its `backtrace` feature: the DWARF symbolizer (gimli/object/addr2line,
// hundreds of KiB) exists so RUST_BACKTRACE=1 can print symbolized panic traces — dead
// weight in the shipped addon, which is built with `panic = "abort"` and can only print the
// panic message + source location before aborting.
//
// Accepted behavior change in shipped binaries: RUST_BACKTRACE=1 prints NO stack frames at
// all — the panic message and source location survive (which is what issue reports contain
// in practice).
//
// `panic_abort` must be in the build-std crate list because the release profile's
// `panic = "abort"` selects that runtime; std keeps its default `panic-unwind` feature so
// the rebuilt std differs from the prebuilt one only by the dropped `backtrace` feature.
//
// Opt-in via the release workflow because it needs `RUSTC_BOOTSTRAP=1` to unlock
// `-Z build-std` on the pinned stable toolchain, the `rust-src` component (installed by the
// release workflow), and an explicit `--target`. wasm targets keep the prebuilt std (it
// bundles the toolchain's self-contained wasi-libc); windows-msvc keeps the prebuilt std (a
// matched A/B in rolldown#10177 measured build-std as a small size loss there) — both still
// get the path remap.
if (process.env.OXC_RESOLVER_BUILD_STD === "1" && isRelease) {
  if (!argsOptions.target) {
    console.warn(
      "OXC_RESOLVER_BUILD_STD=1 requires an explicit --target; building with prebuilt std instead",
    );
  } else if (!argsOptions.target.startsWith("wasm") && !argsOptions.target.includes("windows")) {
    process.env.RUSTC_BOOTSTRAP = "1";
    process.env.CARGO_UNSTABLE_BUILD_STD = "std,panic_abort";
    process.env.CARGO_UNSTABLE_BUILD_STD_FEATURES = "panic-unwind";
  }
}

// Injected config first: cargo applies later `--config` values with higher precedence,
// so a caller-passed `--config` can still override the remap entry.
const cargoOptions = [...(remapConfig ? ["--config", remapConfig] : []), ...restCargoOptions];

const napiArgs = {
  ...argsOptions,
  ...(cargoOptions.length > 0 ? { cargoOptions } : {}),
  cwd: workspaceRoot,
  manifestPath: "napi/Cargo.toml",
  platform: true,
};

console.info("napi build args:", napiArgs);

const { task } = await new NapiCli().build(napiArgs);
await task;

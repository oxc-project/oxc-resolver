import { spawnSync } from "node:child_process";
import * as path from "node:path";

import { assert, test } from "vite-plus/test";

import { normalizePath } from "./utils.mjs";

const rootDir = path.resolve(import.meta.dirname, "../..");
const fixture = path.join(rootDir, "fixtures/package-map/resolution");
const childFixture = path.join(rootDir, "fixtures/package-map/node-options.mjs");
const binding = path.join(
  rootDir,
  process.env.WASI_TEST ? "napi/resolver.wasi.cjs" : "napi/index.js",
);

function nodeOptions(packageMapPath) {
  const escapedPath = packageMapPath.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
  return `--experimental-package-map="${escapedPath}"`;
}

function run(operations) {
  const payload = JSON.stringify({ operations });
  const result = spawnSync(process.execPath, [childFixture, binding, payload], {
    cwd: rootDir,
    encoding: "utf8",
    env: { ...process.env, NODE_OPTIONS: "" },
  });

  assert.equal(result.status, 0, result.stderr || result.stdout);
  return JSON.parse(result.stdout);
}

function assertResolution(result, expected) {
  assert.equal(result.error, undefined);
  assert.equal(normalizePath(result.path), normalizePath(expected));
}

test("resolves from the package map after clearing the cache", () => {
  const importer = path.join(fixture, "apps/web/src");
  const packageMapOptions = nodeOptions(path.join(fixture, "node_modules/.package-map.json"));
  const operations = [
    { nodeOptions: packageMapOptions, importer, specifier: "plain-file" },
    { clearCache: true, importer, specifier: "plain-file" },
  ];
  const [mapped, reloaded] = run(operations);

  const expected = path.join(fixture, "node_modules/store/plain-file.js");
  assertResolution(mapped, expected);
  assertResolution(reloaded, expected);
});

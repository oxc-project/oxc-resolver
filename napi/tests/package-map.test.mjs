import { spawnSync } from "node:child_process";
import * as path from "node:path";

import { assert, test } from "vite-plus/test";

import { normalizePath } from "./utils.mjs";

const rootDir = path.resolve(import.meta.dirname, "../..");
const fixture = path.join(rootDir, "fixtures/package-map/resolution");
const childFixture = path.join(rootDir, "fixtures/package-map/node-options.mjs");

test("enables package maps from NODE_OPTIONS", () => {
  const binding = path.join(
    rootDir,
    process.env.WASI_TEST ? "napi/resolver.wasi.cjs" : "napi/index.js",
  );
  const result = spawnSync(
    process.execPath,
    [
      childFixture,
      binding,
      path.join(fixture, "node_modules/.package-map.json"),
      path.join(fixture, "apps/web/src"),
    ],
    {
      cwd: rootDir,
      encoding: "utf8",
      env: { ...process.env, NODE_OPTIONS: "" },
    },
  );

  assert.equal(result.status, 0, result.stderr || result.stdout);
  assert.match(normalizePath(result.stdout), /\/node_modules\/store\/axios\/index\.js$/);
});

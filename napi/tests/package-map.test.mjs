import { spawnSync } from "node:child_process";
import * as fs from "node:fs";
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
  return [
    "--trace-warnings",
    "--experimental-package-map=ignored.json",
    `--experimental-package-map="${escapedPath}"`,
  ].join(" ");
}

function run(operations, options) {
  const payload = JSON.stringify({ options, operations });
  const result = spawnSync(process.execPath, [childFixture, binding, payload], {
    cwd: rootDir,
    encoding: "utf8",
    env: { ...process.env, NODE_OPTIONS: "" },
  });

  assert.equal(result.status, 0, result.stderr || result.stdout);
  return JSON.parse(result.stdout);
}

function resolve(
  importer,
  specifier,
  packageMapPath = path.join(fixture, "node_modules/.package-map.json"),
) {
  return run([{ nodeOptions: nodeOptions(packageMapPath), importer, specifier }])[0];
}

function assertResolution(result, expected) {
  assert.equal(result.error, undefined);
  assert.equal(normalizePath(result.path), normalizePath(expected));
}

test("resolves package-map dependencies, subpaths, imports, and package self references", () => {
  const importer = path.join(fixture, "apps/web/src");
  for (const [specifier, expected] of [
    ["axios", "node_modules/store/axios/index.js"],
    ["axios/client", "node_modules/store/axios/lib/client.js"],
    ["@bench/ui", "packages/ui/src/index.js"],
    ["@bench/web", "apps/web/src/index.js"],
    ["#react", "node_modules/store/react/index.js"],
    ["plain-directory", "node_modules/store/plain-directory/index.js"],
    ["plain-file", "node_modules/store/plain-file.js"],
  ]) {
    assertResolution(resolve(importer, specifier), path.join(fixture, expected));
  }

  assertResolution(
    resolve(path.join(fixture, "node_modules/store/axios/lib"), "follow-redirects"),
    path.join(fixture, "node_modules/store/follow-redirects/index.js"),
  );
  assertResolution(
    resolve(path.join(fixture, "node_modules/importer"), "plain-file"),
    path.join(fixture, "node_modules/store/plain-file.js"),
  );
});

test("normalizes importer paths before finding their package", () => {
  const importer = [fixture, "packages", "ui", "..", "..", "apps", "web", "src"].join(path.sep);
  assertResolution(
    resolve(importer, "axios"),
    path.join(fixture, "node_modules/store/axios/index.js"),
  );
});

test("reports package-map ownership and dependency errors", () => {
  const ownershipFixture = path.join(rootDir, "fixtures/package-map/find-package-id");
  const packageMapPath = path.join(ownershipFixture, ".package-map.json");

  assert.match(
    resolve(path.join(ownershipFixture, "packages/duplicate"), "dependency", packageMapPath).error,
    /multiple packages/,
  );
  assert.match(
    resolve(path.join(ownershipFixture, "external"), "dependency", packageMapPath).error,
    /not within any package/,
  );
  assert.match(
    resolve(path.join(fixture, "apps/web/src"), "missing-target").error,
    /Package key "missing-target" referenced in dependencies but not defined/,
  );
  assert.match(
    resolve(path.join(fixture, "apps/web/src"), "follow-redirects").error,
    /Cannot find module 'follow-redirects'/,
  );
});

test("rejects invalid package maps", () => {
  const invalidFixture = path.join(rootDir, "fixtures/package-map/invalid");
  for (const [filename, expected] of [
    [".package-map.json", /JSONError/],
    ["invalid-shape.package-map.json", /Invalid package-map\.json/],
    ["empty-url.package-map.json", /empty "url" field/],
    ["invalid-dependencies.package-map.json", /Invalid package-map\.json/],
    ["invalid-url.package-map.json", /unsupported URL scheme/],
    ["invalid-percent.package-map.json", /invalid file URL/],
    ["encoded-separator.package-map.json", /invalid file URL/],
  ]) {
    const result = resolve(invalidFixture, "dependency", path.join(invalidFixture, filename));
    assert.match(result.error, expected);
  }
});

test("reloads NODE_OPTIONS after clearing the resolver cache", () => {
  const importer = path.join(fixture, "apps/web/src");
  const packageMapOptions = nodeOptions(path.join(fixture, "node_modules/.package-map.json"));
  const [mapped, cached, cleared, reloaded] = run([
    { nodeOptions: packageMapOptions, importer, specifier: "plain-file" },
    { nodeOptions: "--trace-warnings", importer, specifier: "plain-file" },
    { clearCache: true, importer, specifier: "plain-file" },
    { nodeOptions: packageMapOptions, clearCache: true, importer, specifier: "plain-file" },
  ]);

  const expected = path.join(fixture, "node_modules/store/plain-file.js");
  assertResolution(mapped, expected);
  assertResolution(cached, expected);
  assert.match(cleared.error, /Cannot find module 'plain-file'/);
  assertResolution(reloaded, expected);
});

test("resolves tsconfig extends through package maps", () => {
  const importer = path.join(fixture, "apps/web/src");
  const packageMapOptions = nodeOptions(path.join(fixture, "node_modules/.package-map.json"));

  for (const configFile of ["tsconfig.package-map.json", "tsconfig.package-map-self.json"]) {
    const [result] = run([{ nodeOptions: packageMapOptions, importer, specifier: "./index.js" }], {
      conditionNames: ["node", "require"],
      tsconfig: { configFile: path.join(fixture, "apps/web", configFile) },
    });
    assertResolution(result, path.join(importer, "index.js"));
  }
});

test("resolves package URLs from the canonical package-map path", () => {
  const symlinkFixture = path.join(rootDir, "fixtures/integration/nested-symlink");
  const toolingLink = path.join(symlinkFixture, "apps/tooling");
  if (!fs.existsSync(toolingLink) || !fs.lstatSync(toolingLink).isSymbolicLink()) {
    return;
  }

  assertResolution(
    resolve(
      path.join(toolingLink, "typescript-config"),
      "dep",
      path.join(toolingLink, ".package-map.json"),
    ),
    path.join(symlinkFixture, "nm/index.js"),
  );
});

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

function run(operations, runtimeArgs = [], scriptArgs = []) {
  const payload = JSON.stringify({ operations });
  const result = spawnSync(
    process.execPath,
    [...runtimeArgs, childFixture, binding, payload, ...scriptArgs],
    {
      cwd: rootDir,
      encoding: "utf8",
      env: { ...process.env, NODE_OPTIONS: "" },
    },
  );

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

for (const runtimeArgs of [
  [
    "--title=resolver-test",
    "--experimental-package-map=fixtures/package-map/resolution/node_modules/.package-map.json",
  ],
  [
    "--require",
    "node:fs",
    "--experimental-package-map",
    path.join(fixture, "node_modules/.package-map.json"),
  ],
]) {
  test.runIf(
    process.allowedNodeEnvironmentFlags.has("--experimental-package-map") && !process.env.WASI_TEST,
  )(`resolves with CLI options: ${runtimeArgs.join(" ")}`, () => {
    const importer = path.join(fixture, "apps/web/src");
    const [result] = run(
      [{ nodeOptions: nodeOptions("missing.json"), importer, specifier: "plain-file" }],
      ["--experimental-package-map=missing.json", ...runtimeArgs],
    );
    const expected = path.join(fixture, "node_modules/store/plain-file.js");
    assertResolution(result, expected);
  });
}

test("preserves the runtime package map after changing process.title", () => {
  const operations = [undefined, "sync", "default", "clone"].map((api) => ({
    api,
    nodeOptions: nodeOptions("missing.json"),
    importer: path.join(fixture, "apps/web/src"),
    specifier: "plain-file",
  }));
  operations[0].execArgv = [
    "--experimental-package-map=missing.json",
    "--experimental-package-map=fixtures/package-map/resolution/node_modules/.package-map.json",
  ];
  operations[0].processTitle = "resolver-test";
  for (const result of run(operations)) {
    assertResolution(result, path.join(fixture, "node_modules/store/plain-file.js"));
  }
});

test("rejects an empty runtime package map instead of falling back to NODE_OPTIONS", () => {
  for (const execArgv of [
    ["--experimental-package-map=valid.json", "--experimental-package-map="],
    ["--experimental-package-map"],
  ]) {
    assert.throws(
      () =>
        run([
          {
            execArgv,
            nodeOptions: nodeOptions(path.join(fixture, "node_modules/.package-map.json")),
            importer: path.join(fixture, "apps/web/src"),
            specifier: "plain-file",
          },
        ]),
      /--experimental-package-map requires a path/,
    );
  }
});

for (const runtimeArgs of [[], ["--"]]) {
  test(`ignores script arguments after ${runtimeArgs.join(" ") || "the entry point"}`, () => {
    const [result] = run(
      [
        {
          nodeOptions: nodeOptions(path.join(fixture, "node_modules/.package-map.json")),
          importer: path.join(fixture, "apps/web/src"),
          specifier: "plain-file",
        },
      ],
      runtimeArgs,
      ["--experimental-package-map=missing.json", "--experimental-package-map", "missing.json"],
    );
    assertResolution(result, path.join(fixture, "node_modules/store/plain-file.js"));
  });
}

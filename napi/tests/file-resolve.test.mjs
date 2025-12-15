import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { assert, test } from "vitest";

import { ModuleType, ResolverFactory } from "../index.js";

const currentDir = join(fileURLToPath(import.meta.url), "..");
const cwd = join(currentDir, "..");
const rootDir = join(cwd, "..");
const fixturesDir = join(rootDir, "fixtures");

test("resolveFileSync basic resolution", () => {
  const resolver = new ResolverFactory();
  const testFile = join(currentDir, "simple.test.mjs");

  const result = resolver.resolveFileSync(testFile, "./resolver.test.mjs");
  assert.equal(result.path, join(currentDir, "resolver.test.mjs"));
});

test("resolveFileSync with relative import", () => {
  const resolver = new ResolverFactory();
  const testFile = join(cwd, "index.js");

  const result = resolver.resolveFileSync(testFile, "./src/lib.rs");
  assert.equal(result.path, join(cwd, "src", "lib.rs"));
});

test("resolveFileSync error handling", () => {
  const resolver = new ResolverFactory();
  const testFile = join(currentDir, "simple.test.mjs");

  const result = resolver.resolveFileSync(testFile, "./nonexistent-file");
  assert.isNotNull(result.error);
  assert.isAbove(result.error.length, 0);
  assert.isUndefined(result.path);
});

test("resolveFileAsync basic resolution", async () => {
  const resolver = new ResolverFactory();
  const testFile = join(currentDir, "simple.test.mjs");

  const result = await resolver.resolveFileAsync(testFile, "./resolver.test.mjs");
  assert.equal(result.path, join(currentDir, "resolver.test.mjs"));
});

test("resolveFileAsync error handling", async () => {
  const resolver = new ResolverFactory();
  const testFile = join(currentDir, "simple.test.mjs");

  const result = await resolver.resolveFileAsync(testFile, "./nonexistent");
  assert.isNotNull(result.error);
  assert.isAbove(result.error.length, 0);
  assert.isUndefined(result.path);
});

test("resolveFileSync with module type", () => {
  const resolver = new ResolverFactory({ moduleType: true });
  const testFile = join(currentDir, "simple.test.mjs");

  const result = resolver.resolveFileSync(testFile, "../index.js");
  assert.equal(result.path, join(cwd, "index.js"));
  assert.isNotNull(result.moduleType);
});

test("resolveFileSync builtin module", () => {
  const resolver = new ResolverFactory({ builtinModules: true });
  const testFile = join(currentDir, "simple.test.mjs");

  const result = resolver.resolveFileSync(testFile, "node:fs");
  assert.deepEqual(result.builtin, {
    resolved: "node:fs",
    isRuntimeModule: true,
  });
});

test("resolveFileSync builtin module without prefix", () => {
  const resolver = new ResolverFactory({ builtinModules: true });
  const testFile = join(currentDir, "simple.test.mjs");

  const result = resolver.resolveFileSync(testFile, "fs");
  assert.deepEqual(result.builtin, {
    resolved: "node:fs",
    isRuntimeModule: false,
  });
});

test("resolveFileSync with extensions option", () => {
  const resolver = new ResolverFactory({
    extensions: [".js", ".mjs", ".json"],
  });
  const testFile = join(currentDir, "simple.test.mjs");

  const result = resolver.resolveFileSync(testFile, "./simple.test");
  assert.equal(result.path, join(currentDir, "simple.test.mjs"));
});

test("resolveFileAsync with module type", async () => {
  const resolver = new ResolverFactory({ moduleType: true });
  const testFile = join(currentDir, "simple.test.mjs");

  const result = await resolver.resolveFileAsync(testFile, "../index.js");
  assert.equal(result.path, join(cwd, "index.js"));
  assert.isNotNull(result.moduleType);
});

test("sync and async return same results", async () => {
  const resolver = new ResolverFactory();
  const testFile = join(currentDir, "simple.test.mjs");
  const request = "./resolver.test.mjs";

  const syncResult = resolver.resolveFileSync(testFile, request);
  const asyncResult = await resolver.resolveFileAsync(testFile, request);

  assert.deepEqual(syncResult, asyncResult);
});

test("sync and async return same errors", async () => {
  const resolver = new ResolverFactory();
  const testFile = join(currentDir, "simple.test.mjs");
  const request = "./nonexistent-module";

  const syncResult = resolver.resolveFileSync(testFile, request);
  const asyncResult = await resolver.resolveFileAsync(testFile, request);

  assert.equal(syncResult.path, asyncResult.path);
  assert.isDefined(syncResult.error);
  assert.isDefined(asyncResult.error);
});

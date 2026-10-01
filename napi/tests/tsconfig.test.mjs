import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { assert, test } from "vite-plus/test";

import { ResolverFactory } from "../index.js";

const currentDir = join(fileURLToPath(import.meta.url), "..");
const fixturesDir = join(currentDir, "..", "..", "fixtures", "tsconfig");

test("resolves the tsconfig for a source file", () => {
  const resolver = new ResolverFactory({ tsconfig: "auto" });
  const result = resolver.findTsconfigSync(join(fixturesDir, "main.ts"));

  assert.isNotNull(result);
  assert.deepEqual(result.tsconfig.compilerOptions.paths, {
    "ts-path": [join(fixturesDir, "src", "foo.js")],
  });
  assert.deepEqual(result.tsconfigPaths, [join(fixturesDir, "tsconfig.json")]);
});

test("asynchronously uses an explicit tsconfig", async () => {
  const tsconfigPath = join(fixturesDir, "cases", "extends-chain", "tsconfig.json");
  const resolver = new ResolverFactory({
    tsconfig: { configFile: tsconfigPath, references: "auto" },
  });
  const result = await resolver.findTsconfigAsync(join(fixturesDir, "main.ts"));

  assert.isNotNull(result);
  assert.equal(result.tsconfig.compilerOptions.experimentalDecorators, true);
  assert.equal(result.tsconfig.compilerOptions.target, "ES2022");
  assert.equal(result.tsconfig.compilerOptions.module, "ESNext");
  assert.deepEqual(result.tsconfigPaths, [tsconfigPath]);
});

test("returns supported compiler options", () => {
  const tsconfigDir = join(fixturesDir, "cases", "extends");
  const tsconfigPath = join(tsconfigDir, "tsconfig.json");
  const resolver = new ResolverFactory({ tsconfig: { configFile: tsconfigPath } });
  const result = resolver.findTsconfigSync(join(fixturesDir, "main.ts"));

  assert.isNotNull(result);
  assert.include(result.tsconfig.compilerOptions, {
    strict: true,
    strictNullChecks: false,
    outDir: join(tsconfigDir, "dist"),
    declarationDir: join(tsconfigDir, "types"),
    resolveJsonModule: true,
    checkJs: false,
  });
});

test("returns null when no tsconfig exists", () => {
  const resolver = new ResolverFactory({ tsconfig: "auto" });
  const result = resolver.findTsconfigSync(
    join(fixturesDir, "..", "integration", "misc", "index.js"),
  );

  assert.isNull(result);
});

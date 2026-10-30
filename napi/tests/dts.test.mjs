import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { assert, test } from "vite-plus/test";

import { ResolverFactory } from "../index.js";

const currentDir = join(fileURLToPath(import.meta.url), "..");
const rootDir = join(currentDir, "..", "..");
const pnpmDir = join(rootDir, "fixtures", "pnpm");

const resolver = new ResolverFactory({
  conditionNames: ["import", "types"],
});

test("magic-string resolves to .d.mts", () => {
  const containingFile = join(pnpmDir, "index.ts");
  const result = resolver.resolveDtsSync(containingFile, "magic-string");
  assert.isUndefined(result.error);
  assert.match(result.path, /magic-string\.es\.d\.mts$/);
});

test("typesVersions option", () => {
  const containingFile = join(rootDir, "fixtures", "dts-resolver", "index.ts");
  const resolve = (typesVersions) =>
    new ResolverFactory({ typesVersions })
      .resolveDtsSync(containingFile, "ranged-types-versions")
      .path.split(/[\\/]ranged-types-versions[\\/]/)[1];
  assert.equal(resolve(undefined), "ts5.6/index.d.ts");
  assert.equal(resolve(true), "ts5.6/index.d.ts");
  assert.equal(resolve(false), "index.d.ts");
  assert.equal(resolve("latest"), "index.d.ts");
  assert.equal(resolve("5.7.2"), "ts5.7/index.d.ts");
  assert.equal(resolve("6.0.3"), "index.d.ts");
});

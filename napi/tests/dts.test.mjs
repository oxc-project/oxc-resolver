import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { assert, test } from "vitest";

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

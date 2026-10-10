import * as path from "node:path";
import { assert, describe, it } from "vite-plus/test";

import { ResolverFactory } from "../index.js";
import { normalizePath } from "./utils.mjs";

const fixtureDir = path.resolve(
  import.meta.dirname,
  "../../fixtures/enhanced-resolve/test/fixtures",
);

describe("option", () => {
  describe("tsconfig", () => {
    it("applies inherited custom conditions", () => {
      const root = path.resolve(
        import.meta.dirname,
        "../../fixtures/tsconfig/cases/custom-conditions",
      );
      const resolver = new ResolverFactory({ tsconfig: "auto", conditionNames: ["global"] });
      const result = resolver.resolveFileSync(
        path.join(root, "inherited/main.ts"),
        "custom-conditions-pkg",
      );
      assert.equal(result.path, path.join(root, "node_modules/custom-conditions-pkg/tsconfig.js"));
    });

    it("supports the explicit nearest-config fallback mode", () => {
      const root = path.resolve(
        import.meta.dirname,
        "../../fixtures/tsconfig/cases/solution-nearest-fallback",
      );
      const resolver = new ResolverFactory({ tsconfig: "auto-nearest", extensions: [".ts"] });
      const result = resolver.resolveFileSync(path.join(root, "stories/story.ts"), "@app/util");
      assert.equal(result.path, path.join(root, "src/libs/util.ts"));
    });

    it("rejects unknown discovery strings", () => {
      assert.throws(
        () => new ResolverFactory({ tsconfig: "nearest" }),
        /not a valid tsconfig discovery mode/,
      );
    });
  });

  describe("aliasFields", () => {
    it("should allow field string ", () => {
      const resolver = new ResolverFactory({ aliasFields: ["browser"] });
      assert.match(
        normalizePath(resolver.sync(fixtureDir, "./browser-module/lib/replaced.js").path),
        /browser-module\/lib\/browser\.js$/,
      );
    });
    it("should allow json path array", () => {
      const resolver = new ResolverFactory({
        aliasFields: [["innerBrowser1", "field", "browser"]],
      });

      assert.match(
        normalizePath(resolver.sync(fixtureDir, "./browser-module/lib/main1.js").path),
        /browser-module\/lib\/main\.js$/,
      );
    });
  });

  describe("exportsFields", () => {
    const createTest = (exportsFields) => {
      const resolver = new ResolverFactory({ exportsFields });
      assert.match(
        normalizePath(
          resolver.sync(path.resolve(fixtureDir, "./exports-field3"), "exports-field").path,
        ),
        /\/exports-field\/src\/index\.js$/,
      );
    };
    it("should allow string as field item", () => createTest(["broken"]));
    it("should allow json path array as field item", () => createTest([["broken"]]));
  });

  describe("mainFields", () => {
    const createTest = (mainFields) => {
      const resolver = new ResolverFactory({ mainFields });
      assert.match(normalizePath(resolver.sync(fixtureDir, "../..").path), /\/lib\/index\.js$/);
    };
    it("should use `'main'` as default", () => createTest(undefined));
    it("should allow field string", () => createTest("main"));
    it("should allow field array", () => createTest(["main"]));
  });
});

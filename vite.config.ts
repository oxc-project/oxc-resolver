import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  lint: { options: { typeAware: false, typeCheck: false } },
  fmt: {
    ignorePatterns: [
      "CHANGELOG.md",
      "fixtures",
      "napi/browser.js",
      "napi/browser.js",
      "napi/index.d.ts",
      "napi/index.js",
      "napi/resolver.wasi-browser.js",
      "napi/resolver.wasi.cjs",
      "napi/wasi-worker-browser.mjs",
      "napi/wasi-worker.mjs",
      "napi/webcontainer-fallback.js",
    ],
  },
});

import { pathToFileURL } from "node:url";

const [bindingPath, payloadJson] = process.argv.slice(2);
if (!bindingPath || !payloadJson) {
  throw new Error("Expected binding path and operation payload");
}

const { operations } = JSON.parse(payloadJson);
if (operations[0]?.nodeOptions !== undefined) {
  process.env.NODE_OPTIONS = operations[0].nodeOptions;
}
const binding = await import(pathToFileURL(bindingPath));
const ResolverFactory = binding.ResolverFactory ?? binding.default.ResolverFactory;
const resolver = new ResolverFactory({ conditionNames: ["node", "require"] });
const results = [];

for (const { nodeOptions, clearCache, importer, specifier } of operations) {
  if (nodeOptions !== undefined) {
    process.env.NODE_OPTIONS = nodeOptions;
  }
  if (clearCache) {
    resolver.clearCache();
  }
  results.push(resolver.sync(importer, specifier));
}

process.stdout.write(JSON.stringify(results));

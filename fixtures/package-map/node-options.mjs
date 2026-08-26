import { pathToFileURL } from "node:url";

const [bindingPath, packageMapPath, importer] = process.argv.slice(2);
if (!bindingPath || !packageMapPath || !importer) {
  throw new Error("Expected binding, package-map, and importer paths");
}

const escapedPackageMapPath = packageMapPath.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
process.env.NODE_OPTIONS = [
  "--trace-warnings",
  "--experimental-package-map=ignored.json",
  `--experimental-package-map="${escapedPackageMapPath}"`,
].join(" ");

const binding = await import(pathToFileURL(bindingPath));
const ResolverFactory = binding.ResolverFactory ?? binding.default.ResolverFactory;
const result = new ResolverFactory({
  conditionNames: ["node", "require"],
}).sync(importer, "axios");

if (result.error) {
  throw new Error(result.error);
}
process.stdout.write(result.path);

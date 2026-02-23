import { readFile, writeFile } from "node:fs/promises";

const buildPath = new URL("../public/build.json", import.meta.url);

let buildJson = { buildNumber: 0 };
try {
  buildJson = JSON.parse(await readFile(buildPath, "utf8"));
} catch {
  buildJson = { buildNumber: 0 };
}

const nextBuildNumber = (buildJson.buildNumber || 0) + 1;
buildJson.buildNumber = nextBuildNumber;

await writeFile(buildPath, `${JSON.stringify(buildJson, null, 2)}\n`);

console.log(`Build number: ${nextBuildNumber}`);

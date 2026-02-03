import { Glob } from "bun";
import { cp } from "node:fs/promises";
import { rename } from "node:fs/promises";
import { existsSync, mkdirSync, writeFileSync, readFileSync } from "node:fs";

// Parse command line arguments
const args = process.argv.slice(2);
const target = args.findIndex(arg => arg === "--target") >= 0 
  ? args[args.findIndex(arg => arg === "--target") + 1] 
  : "linux";

// Build-time constants
const BUILD_TIME = new Date().toISOString();
const VERSION = process.env.npm_package_version || "1.0.0";

async function build() {
  console.log(`Building keygenerator v${VERSION}...`);
  console.log(`Target: ${target}`);
  console.log(`Build time: ${BUILD_TIME}`);

  // Ensure dist directory exists
  if (!existsSync("./dist")) {
    mkdirSync("./dist", { recursive: true });
  }

  // Bundle TypeScript without compiling to standalone
  const result = await Bun.build({
    entrypoints: ["./src/index.ts"],
    outdir: "./dist",
    minify: true,
    sourcemap: "linked",
    bytecode: true,
    external: ["electron"],
    define: {
      "process.env.NODE_ENV": JSON.stringify("production"),
      "BUILD_TIME": JSON.stringify(BUILD_TIME),
      "VERSION": JSON.stringify(VERSION),
    },
  });

  if (!result.success) {
    console.error("Build failed:");
    for (const log of result.logs) {
      console.error(log);
    }
    process.exit(1);
  }

  // Rename index.js to main.js for electron-builder
  const indexPath = "./dist/index.js";
  const mainPath = "./dist/main.js";
  
  if (existsSync(indexPath)) {
    await rename(indexPath, mainPath);
    console.log("Renamed dist/index.js to dist/main.js");
  }

  // Copy package.json to dist with updated main field
  const pkg = JSON.parse(readFileSync("./package.json", "utf-8"));
  pkg.main = "main.js";
  delete pkg.scripts;
  delete pkg.devDependencies;
  delete pkg.peerDependencies;
  writeFileSync("./dist/package.json", JSON.stringify(pkg, null, 2));
  console.log("Copied package.json to dist/");

  // Copy UI assets
  await cp("./src/ui", "./dist/ui", { recursive: true });
  
  // Copy auth preload script
  await cp("./src/preload.js", "./dist/preload.js", { recursive: true });
}

build().catch(console.error);

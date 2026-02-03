import { existsSync, mkdirSync, copyFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

// Build-time constants
//const VERSION = process.env.npm_package_version || "1.0.0";

/**
 * Recursively copy a directory from src to dest
 */
function copyDirectory(src: string, dest: string): void {
  // Ensure destination directory exists
  if (!existsSync(dest)) {
    mkdirSync(dest, { recursive: true });
  }

  // Read all files in the source directory
  const entries = readdirSync(src);

  for (const entry of entries) {
    const srcPath = join(src, entry);
    const destPath = join(dest, entry);
    const stat = statSync(srcPath);

    if (stat.isDirectory()) {
      // Recursively copy subdirectories
      copyDirectory(srcPath, destPath);
    } else {
      // Copy files
      copyFileSync(srcPath, destPath);
    }
  }
}

async function build() {
  // Ensure dist directory exists
  if (!existsSync("./dist")) {
    mkdirSync("./dist", { recursive: true });
  }

  // Bundle TypeScript to JavaScript
  const result = await Bun.build({
    entrypoints: ["./src/index.ts"],
    outdir: "./dist",
    minify: false,
    sourcemap: "linked",
    target: "node",
    external: ["electron"],
  });

  if (!result.success) {
    console.error("Build failed:");
    for (const log of result.logs) {
      console.error(log);
    }
    process.exit(1);
  }

  // Copy UI files to dist directory
  console.log("Copying UI files...");
  if (existsSync("./src/ui")) {
    copyDirectory("./src/ui", "./dist/ui");
    console.log("UI files copied successfully.");
  } else {
    console.error("Warning: ./src/ui directory not found");
  }

  // Copy preload.js to dist directory
  if (existsSync("./src/preload.js")) {
    copyFileSync("./src/preload.js", "./dist/preload.js");
    console.log("Preload file copied successfully.");
  } else {
    console.error("Warning: ./src/preload.js not found");
  }
}

build().catch(console.error);

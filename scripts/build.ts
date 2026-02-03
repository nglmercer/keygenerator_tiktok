import { existsSync, mkdirSync } from "node:fs";
// Build-time constants
//const VERSION = process.env.npm_package_version || "1.0.0";

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
}

build().catch(console.error);

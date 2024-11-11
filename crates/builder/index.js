import build from "./lib.js";
import fs from "node:fs/promises";

(async () => {
    const __cwd = process.cwd();

    const start = performance.now();
    await build(JSON.parse(await fs.readFile(`${__cwd}/builder.json`)));
    console.log(`took ${Math.floor(performance.now() - start)}ms`);
})();

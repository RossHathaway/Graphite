// Render branding/mark.svg to the PNG favicons frontend/index.html links, and
// the desktop app icon. Uses the Playwright Chromium the build machines have.
//   node designer-branding/render_pngs.mjs
import { createRequire } from "node:module";
const { chromium } = createRequire(import.meta.url)("playwright");
import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const mark = readFileSync(join(root, "branding/mark.svg"), "utf8");
const sizes = {
	"favicon-16x16.png": 16,
	"favicon-32x32.png": 32,
	"apple-touch-icon.png": 180,
	"android-chrome-192x192.png": 192,
	"android-chrome-512x512.png": 512,
	"icon-256.png": 256,
};

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: 1 });
for (const [file, size] of Object.entries(sizes)) {
	await page.setViewportSize({ width: size, height: size });
	const sized = mark.replace(/width="\d+" height="\d+"/, `width="${size}" height="${size}"`);
	await page.setContent(`<html><body style="margin:0;background:transparent">${sized}</body></html>`);
	const png = await page.locator("svg").screenshot({ omitBackground: true });
	writeFileSync(join(root, "branding/favicons", file), png);
}
await browser.close();

// favicon.ico and the Windows app icon, from the rendered PNGs (Pillow).
const py = `
from PIL import Image
import sys
img = Image.open(sys.argv[1] + "/branding/favicons/icon-256.png")
img.save(sys.argv[1] + "/branding/favicons/favicon.ico", sizes=[(16, 16), (32, 32), (48, 48)])
import os
os.makedirs(sys.argv[1] + "/branding/app-icons", exist_ok=True)
img.save(sys.argv[1] + "/branding/app-icons/graphite.ico", sizes=[(16, 16), (32, 32), (48, 48), (256, 256)])
os.remove(sys.argv[1] + "/branding/favicons/icon-256.png")
`;
execFileSync("python3", ["-c", py, root], { stdio: "inherit" });
console.log("wrote favicons and app icon");

import { chromium } from '@playwright/test';
const browser = await chromium.launch({ headless: true });
try {
  for (const size of [192, 512]) {
    const page = await browser.newPage({
      viewport: { width: size, height: size },
      deviceScaleFactor: 1,
    });
    await page.setContent(
      `<style>html,body{margin:0;width:100%;height:100%;background:#171b1e}svg{width:100%;height:100%}</style><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><rect width="100" height="100" rx="20" fill="#171b1e"/><path d="M23 80V43a27 27 0 0 1 54 0v37L63.5 71.5 50 80 36.5 71.5 23 80Z" fill="none" stroke="#e7aa70" stroke-width="4.5"/><path d="M40 40v8m20-8v8" stroke="#e7aa70" stroke-width="6" stroke-linecap="round"/></svg>`,
    );
    await page.screenshot({ path: `web/public/icon-${size}.png` });
    await page.close();
  }
} finally {
  await browser.close();
}

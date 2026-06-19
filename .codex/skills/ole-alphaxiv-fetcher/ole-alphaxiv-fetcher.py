#!/usr/bin/env python3
"""
ole-alphaxiv-fetcher: Fetch and parse papers from alphaxiv.org

Usage:
    python3 ole-alphaxiv-fetcher.py <URL>
    python3 ole-alphaxiv-fetcher.py 2605.06647  # auto-constructs URL

Output files in current directory:
    {paper_id}_screenshot.png  - Page screenshot
    {paper_id}_page.html       - Full page HTML
    {paper_id}_full.txt        - Extracted PDF text
    {paper_id}_meta.json       - Paper metadata
"""

import asyncio
import json
import os
import platform
import sys
import tempfile
import urllib.request
import urllib.parse


def _find_browser():
    """Find available Chromium browser. Returns None to use Playwright bundled."""
    system = platform.system()
    if system == "Darwin":
        paths = [
            "/Applications/Helium.app/Contents/MacOS/Helium",
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    elif system == "Linux":
        paths = [
            os.environ.get("PLAYWRIGHT_CHROMIUM_PATH", ""),
            "/usr/bin/chromium-browser",
            "/usr/bin/chromium",
            "/usr/bin/google-chrome",
            "/usr/bin/google-chrome-stable",
            "/snap/bin/chromium",
        ]
    else:
        return None
    for p in paths:
        if p and os.path.exists(p):
            return p
    return None


def normalize_url(raw: str) -> tuple[str, str]:
    """Convert input to (url, paper_id)."""
    if raw.strip().isdigit() or ("." in raw and "/" not in raw):
        paper_id = raw.strip()
        return f"https://www.alphaxiv.org/abs/{paper_id}", paper_id

    url = raw.strip()
    if not url.startswith("http"):
        url = "https://" + url

    parsed = urllib.parse.urlparse(url)
    parts = [p for p in parsed.path.split("/") if p]
    paper_id = parts[-1] if parts else "unknown"
    return url, paper_id


async def fetch_paper(url: str, paper_id: str) -> dict:
    from playwright.async_api import async_playwright

    pdf_urls = []
    title = ""
    abstract = ""

    browser_path = _find_browser()

    async with async_playwright() as p:
        launch_args = {
            "headless": True,
            "args": ["--no-sandbox", "--disable-blink-features=AutomationControlled"],
        }
        if browser_path:
            launch_args["executable_path"] = browser_path
            print(f"     Using browser: {browser_path}")
        else:
            print("     Using Playwright bundled Chromium")

        browser = await p.chromium.launch(**launch_args)
        context = await browser.new_context(viewport={"width": 1440, "height": 900})
        page = await context.new_page()

        async def _on_response(response):
            if response.url.endswith(".pdf"):
                pdf_urls.append(response.url)

        page.on("response", _on_response)

        # 1. Visit abstract page
        print(f"[1/6] Visiting {url}")
        await page.goto(url, wait_until="networkidle", timeout=60000)
        await page.wait_for_timeout(2000)

        # 2. Extract metadata
        title = await page.title()

        abstract = await page.evaluate("""
            () => {
                const script = document.querySelector('script[type="application/ld+json"]');
                if (script) {
                    try {
                        const data = JSON.parse(script.textContent);
                        if (data.abstract) return data.abstract;
                    } catch(e) {}
                }
                const p = document.querySelector('p, [class*="abstract"], [class*="summary"]');
                return p ? p.textContent.substring(0, 2000) : "";
            }
        """)

        authors = await page.evaluate("""
            () => {
                const script = document.querySelector('script[type="application/ld+json"]');
                if (script) {
                    try {
                        const data = JSON.parse(script.textContent);
                        if (Array.isArray(data.author)) {
                            return data.author.map(a => a.name);
                        }
                    } catch(e) {}
                }
                return [];
            }
        """)

        # Save HTML
        page_html = await page.content()
        html_path = f"{paper_id}_page.html"
        with open(html_path, "w") as f:
            f.write(page_html)
        print(f"[2/6] HTML saved: {html_path}")

        # 3. Screenshot
        ss_path = f"{paper_id}_screenshot.png"
        await page.screenshot(path=ss_path, full_page=True)
        print(f"[3/6] Screenshot saved: {ss_path}")

        await browser.close()

    # 4. Download PDF
    if not pdf_urls:
        pdf_urls.append(f"https://papers-pdfs.assets.alphaxiv.org/{paper_id}v1.pdf")

    pdf_url = pdf_urls[0]
    print(f"[4/6] Downloading PDF: {pdf_url}")

    tmp_pdf = tempfile.mktemp(suffix=".pdf")
    try:
        urllib.request.urlretrieve(pdf_url, tmp_pdf)
        pdf_size = os.path.getsize(tmp_pdf) / 1024
        print(f"     PDF size: {pdf_size:.0f} KB")
    except Exception as e:
        print(f"     PDF download failed: {e}")
        tmp_pdf = None

    # 5. Parse PDF
    full_text = ""
    num_pages = 0
    if tmp_pdf and os.path.exists(tmp_pdf):
        try:
            from pypdf import PdfReader

            print("[5/6] Parsing PDF...")
            reader = PdfReader(tmp_pdf)
            num_pages = len(reader.pages)
            print(f"     Pages: {num_pages}")

            for i, pg in enumerate(reader.pages):
                text = pg.extract_text() or ""
                full_text += text + "\n\n"

            txt_path = f"{paper_id}_full.txt"
            with open(txt_path, "w") as f:
                f.write(full_text)
            print(f"[5/6] Full text saved: {txt_path} ({len(full_text)} chars)")
        except Exception as e:
            print(f"     PDF parsing failed: {e}")

        os.unlink(tmp_pdf)

    # 6. Save metadata
    meta = {
        "title": title,
        "authors": authors,
        "abstract": abstract,
        "paper_id": paper_id,
        "pdf_url": pdf_url,
        "source_url": url,
        "num_pages": num_pages,
        "text_length": len(full_text),
    }

    meta_path = f"{paper_id}_meta.json"
    with open(meta_path, "w") as f:
        json.dump(meta, f, indent=2, ensure_ascii=False)
    print(f"[6/6] Metadata saved: {meta_path}")

    return meta


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 ole-alphaxiv-fetcher.py <URL or arxiv_id>")
        print("Example: python3 ole-alphaxiv-fetcher.py 2605.06647")
        print("         python3 ole-alphaxiv-fetcher.py https://www.alphaxiv.org/abs/2605.06647")
        sys.exit(1)

    raw = sys.argv[1]
    url, paper_id = normalize_url(raw)
    print(f"Paper ID: {paper_id}")
    print(f"URL: {url}")
    print("=" * 60)

    meta = asyncio.run(fetch_paper(url, paper_id))

    print("\n" + "=" * 60)
    print(f"Title: {meta['title']}")
    print(f"Authors: {', '.join(meta['authors'][:5])}" + (f" et al. ({len(meta['authors'])} total)" if len(meta['authors']) > 5 else ""))
    print(f"Pages: {meta['num_pages']}")
    print(f"Text length: {meta['text_length']} chars")
    if meta["abstract"]:
        print(f"\nAbstract: {meta['abstract'][:300]}...")
    print("=" * 60)


if __name__ == "__main__":
    main()

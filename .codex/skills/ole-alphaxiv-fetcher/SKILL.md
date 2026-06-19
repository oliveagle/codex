---
name: ole-alphaxiv-fetcher
description: "Fetch and parse papers from alphaxiv.org using headless browser. Downloads PDF, extracts full text, saves screenshot and metadata. Trigger on: alphaxiv, fetch paper, download paper, parse paper from alphaxiv, get paper text, arxiv paper fetch."
---

# ole-alphaxiv-fetcher

Fetch and parse papers from alphaxiv.org using Helium browser in headless mode.

## When to Use

User asks to:
- Fetch a paper from alphaxiv
- Download and parse an alphaxiv paper
- Get the full text of a paper from alphaxiv
- Summarize an alphaxiv paper (fetch first, then summarize)

## How It Works

1. Launch Helium browser in headless mode via Playwright
2. Visit the alphaxiv abstract page
3. Intercept PDF URL from network traffic
4. Download the PDF and parse full text with pypdf
5. Save: screenshot, HTML, full text, and metadata

## Output Files

| File | Description |
|------|-------------|
| `{paper_id}_screenshot.png` | Page screenshot |
| `{paper_id}_page.html` | Full page HTML |
| `{paper_id}_full.txt` | Extracted PDF text (all pages) |
| `{paper_id}_meta.json` | Title, authors, abstract, PDF URL |

## Execution

Run the fetcher script:

```bash
python3 ~/.agents/skills/ole-alphaxiv-fetcher/ole-alphaxiv-fetcher.py <URL>
```

Example:
```bash
python3 ~/.agents/skills/ole-alphaxiv-fetcher/ole-alphaxiv-fetcher.py https://www.alphaxiv.org/abs/2605.06647
```

## URL Patterns

Accepts these URL formats:
- `https://www.alphaxiv.org/abs/{arxiv_id}`
- `https://alphaxiv.org/abs/{arxiv_id}`
- Plain arxiv ID like `2605.06647` (auto-constructs URL)

## Prerequisites

- Python 3.10+
- `playwright` and `pypdf` installed
- A Chromium browser available (auto-detected on both macOS and Linux)

Install dependencies:
```bash
pip3 install playwright pypdf
python3 -m playwright install chromium  # Linux: installs bundled Chromium
```

## Cross-Platform

The script auto-detects the browser:

| Platform | Browser Priority |
|----------|-----------------|
| macOS | Helium > Google Chrome > Chromium.app |
| Linux | `PLAYWRIGHT_CHROMIUM_PATH` env > chromium-browser > chromium > google-chrome > Playwright bundled |

On Linux, after `playwright install chromium`, the bundled Chromium is used automatically.

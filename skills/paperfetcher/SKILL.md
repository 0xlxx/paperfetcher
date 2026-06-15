---
name: paperfetcher
description: Search for and fetch academic papers using the paperfetcher CLI. Use when you need to research academic literature, download scientific paper PDFs, look up DOI metadata, or batch process academic papers into a local directory.
---

# Paperfetcher CLI

`paperfetcher` is an LLM-agent-friendly CLI tool for searching, retrieving, and managing academic papers. It defaults to structured JSON outputs, prints errors uniformly, and provides atomic commands designed for programmatic pipelines.

## Quick Start

You can invoke `paperfetcher` directly from the terminal. By default, it expects a user email for API politeness.

```bash
# 1. Search for a paper
paperfetcher --email <your-email> search 'quantum computing' --limit 3

# 2. Lookup metadata for a specific DOI
paperfetcher --email <your-email> lookup '10.1038/nature14539'

# 3. Fetch (download) the PDF and its JSON metadata
paperfetcher --email <your-email> fetch '10.1038/nature14539' --with-metadata
```

## Workflows & Best Practices for Agents

### 1. JSON First, No Guesswork
`paperfetcher` outputs raw JSON to `stdout` and human-readable logging to `stderr`. 
- Always parse the `stdout` as JSON. 
- A successful response will have `{"status": "success", ...}`. 
- An error will have `{"status": "error", "error": {"code": "...", "message": "..."}}`.

### 2. Batch Processing with STDIN
If you need to fetch multiple DOIs, DO NOT run a `for` loop invoking `paperfetcher fetch` repeatedly. Use the `--stdin` feature for high-performance concurrent downloads:

```bash
# Good: Batch fetch via stdin
echo -e "10.1038/nature14539\n10.1109/cvpr.2016.90" | paperfetcher --email myemail@example.com fetch --stdin --with-metadata
```

### 3. Check Status First
Before attempting to download a paper, check if it already exists locally using the `status` command:

```bash
paperfetcher --email myemail@example.com status '10.1038/nature14539'
```
If `"has_pdf": true` is returned, the paper is already downloaded locally. You can find its path in `"pdf_path"`.

### 4. Reading Downloaded Papers
By default, downloaded papers and their metadata are stored in the local application data directory (`~/.local/share/paperfetcher/papers/` on Linux, `~/Library/Application Support/paperfetcher/papers/` on macOS).
When you fetch a paper with `--with-metadata`, you can read the `.json` file to get its title, authors, abstract, and then read the `.pdf` file using your standard PDF extraction tools.

## Command Reference

- `search <query> [--limit <N>] [--year <YYYY>]`: Returns a list of matching papers and their DOIs.
- `lookup <doi>`: Returns full metadata for a specific paper.
- `fetch <doi|file> [--stdin] [--with-metadata]`: Downloads the PDF from Open Access sources (like Unpaywall/OpenAlex) and saves it locally.
- `status <doi>`: Checks if a PDF and its metadata exist locally.
- `list`: Lists all locally downloaded papers.
- `remove <doi>`: Deletes the paper from local storage.

## Troubleshooting

- **Config Error (`email is required`)**: Make sure you supply `--email <email>` or set the `PAPERFETCHER_EMAIL` environment variable.
- **Not Found (`not_found`)**: The DOI might not be indexed by the current source, or the paper does not have an Open Access PDF available.
- **Empty Output**: If you need silent operation, you can pass `--quiet` to suppress `stderr` logs.

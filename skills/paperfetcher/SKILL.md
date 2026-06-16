---
name: paperfetcher
description: Search for and fetch academic papers using the paperfetcher CLI. ALWAYS use this skill whenever the user mentions downloading papers, researching academic literature, finding papers by DOI, fetching PDFs from journals (like Nature, Science, IEEE, arXiv), or doing any kind of literature review, even if they don't explicitly mention "paperfetcher" or "CLI".
---

# Paperfetcher CLI

`paperfetcher` is a fast, LLM-agent-friendly CLI tool for searching, retrieving, and managing academic papers. It defaults to structured JSON outputs, prints errors uniformly, and provides atomic commands designed for programmatic pipelines.

## Why use paperfetcher?
Instead of writing custom Python scripts to scrape Google Scholar or manually download PDFs, use `paperfetcher`. It handles multi-source API fallbacks (OpenAlex, Unpaywall, SemanticScholar, CrossRef), concurrent downloads, and structured local indexing out of the box.

## Quick Start

You can invoke `paperfetcher` directly from the terminal. 

```bash
# 1. Search for a paper (returns JSON)
paperfetcher search 'quantum computing' --limit 3

# 2. Lookup metadata for a specific DOI
paperfetcher lookup '10.1038/nature14539'

# 3. Fetch (download) the PDF and its JSON metadata
paperfetcher fetch '10.1038/nature14539' --with-metadata
```

## Workflows & Best Practices for Agents

### 1. JSON First, No Guesswork
`paperfetcher` outputs raw JSON to `stdout` and human-readable logging to `stderr`. 
**Why?** So you don't have to write brittle regex to parse the output.
- Always parse `stdout` as JSON. 
- A successful response will have `{"status": "success", "config": {...}, "message": "...", ...data}`. 
- An error will have `{"status": "error", "error": {"code": "...", "message": "...", "suggestions": [...]}}`.

### 2. High-Performance Batch Processing
**Why?** If you run a bash `for` loop invoking `paperfetcher fetch` repeatedly, it is slow and sequential.
Instead, use the `--stdin` feature for high-performance concurrent downloads. `paperfetcher` will automatically parallelize the downloads.

**Example 1: Batch fetch**
Input: I need to download these two DOIs: 10.1038/nature14539 and 10.1109/cvpr.2016.90
Output:
```bash
echo -e "10.1038/nature14539\n10.1109/cvpr.2016.90" | paperfetcher fetch --stdin --with-metadata
```

### 3. Check Status Before Fetching
**Why?** To save network bandwidth and time, check if the paper already exists locally.

```bash
paperfetcher status '10.1038/nature14539'
```
If `"has_pdf": true` is returned, the paper is already downloaded locally. You can find its absolute path in the `"pdf_path"` field.

### 4. Configuration Management
If you encounter an error like `email is required`, you can set the email globally via the `config` command instead of passing it every time.

```bash
paperfetcher config set --email "your_email@example.com"
```
You can also use `paperfetcher config show` to view the current configurations, including the `data_dir` where papers are stored.

## Command Reference

- `search <query> [--limit <N>] [--year <YYYY>]`: Returns a list of matching papers and their DOIs.
- `lookup <doi>`: Returns full metadata for a specific paper.
- `fetch <doi|file> [--stdin] [--with-metadata]`: Downloads the PDF from Open Access sources and saves it locally.
- `status <doi>`: Checks if a PDF and its metadata exist locally.
- `list`: Lists all locally downloaded papers.
- `remove <doi>`: Deletes the paper from local storage.
- `config [show|set]`: View or modify the global configuration (email, output format, limits, concurrent threads).

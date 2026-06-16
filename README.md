# paperfetcher

`paperfetcher` is a Rust-powered CLI tool designed specifically for LLM Agents (and developers) to search, lookup, and retrieve academic papers. It serves as a modern Rust replacement for the deprecated Python-based `pdfsearch`.

[简体中文](./README_zh.md)

## ▣ Installation

### Via Homebrew (Recommended)
```bash
brew tap 0xlxx/tap
brew install paperfetcher
```

### From Source
```bash
# Clone the repository and install using cargo
git clone https://github.com/0xlxx/paperfetcher.git
cd paperfetcher
cargo install --path .
```

## ▣ Quick Start

```bash
# 1. Search academic papers (defaults to JSON output, perfect for LLM Agents or jq)
paperfetcher search "Attention is all you need" --limit 3

# 2. Lookup metadata of a paper by DOI
paperfetcher lookup 10.1038/nature14539

# 3. Download a paper's PDF to the current directory (stateless, lightweight)
paperfetcher fetch 10.1038/nature14539

# 4. Stream PDF binary directly to stdout (great for piping with paperreader or other tools)
paperfetcher fetch 10.1038/nature14539 --stdout > nature_paper.pdf

# 5. Output CLI schema JSON (for LLM Agents to introspect and auto-select subcommands)
paperfetcher schema
```

## ▣ Motivation & Details

### Why paperfetcher?
- **Replacement for pdfsearch**: This repository officially deprecates and replaces the old Python version `pdfsearch`.
- **LLM Agent-Friendly**: Native support for `--output json` and a `schema` command for seamless agent self-introspection.
- **High Performance & Stateless**: Written in Rust, removing bloated Python dependencies. The redesigned `fetch` operates without stateful indexes, defaulting to `$PWD` for a clean Unix CLI experience.
- **Pipeline-First**: Supports `--stdout` to write binary streams directly, enabling diskless workflow pipelining (e.g. `paperfetcher fetch <doi> --stdout | paperreader`).

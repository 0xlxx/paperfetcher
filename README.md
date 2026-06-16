# paperfetcher

`paperfetcher` 是一个专门为 LLM Agent（以及开发者）设计的学术论文检索与获取 CLI 工具，用 Rust 编写。它是原 Python 项目 `pdfsearch` 的现代 Rust 替代版，用于快速检索学术文献、查询元数据、并一键获取 PDF 全文。

## ▣ Installation / 安装

```bash
# Clone the repository and install using cargo
git clone https://github.com/0xlxx/paperfetcher.git
cd paperfetcher
cargo install --path .
```

## ▣ Quick Start / 快速上手

```bash
# 1. 检索学术论文 (默认以 JSON 格式输出，方便 LLM Agent 或 jq 解析)
paperfetcher search "Attention is all you need" --limit 3

# 2. 查询特定 DOI 论文元数据
paperfetcher lookup 10.1038/nature14539

# 3. 下载 PDF 到当前目录 (无状态、轻量、直接下载)
paperfetcher fetch 10.1038/nature14539

# 4. 下载 PDF 并直接写入 stdout (适合 Unix 管道，配合 paperreader 等后续工具)
paperfetcher fetch 10.1038/nature14539 --stdout > nature_paper.pdf

# 5. 输出 CLI schema JSON (以便 LLM Agent 进行自省以自动选择子命令)
paperfetcher schema
```

## ▣ Motivation & Details / 动机与详情

### 为什么使用 `paperfetcher`？
- **替代旧版 pdfsearch**：本项目已全面替代并废弃了旧的 Python 仓库 `pdfsearch`。
- **Agent 友好**：原生支持 `--output json` 和 `schema` 子命令，方便 LLM Agent 自省与结构化输入输出交互。
- **高性能与无状态**：使用 Rust 编写，摆脱了原 Python 版本复杂的依赖与性能瓶颈；最新重构版本的 `fetch` 彻底解除了全局数据库状态绑定，默认从当前执行目录存取，提供无状态的原生命令行体验。
- **Pipeline 管道协同**：支持 `--stdout` 参数将 PDF 二进制流输出至 stdout，实现与下游工具（如 `paperreader`）通过标准 Unix 管道进行高效的协同处理，无需中间文件落盘。

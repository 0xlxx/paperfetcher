// completions 子命令 — 生成 shell 补全脚本
use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::Cli;

/// 执行补全脚本生成命令
///
/// 将指定 shell 的补全脚本输出到 stdout
pub fn execute(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "paperfetcher", &mut std::io::stdout());
}

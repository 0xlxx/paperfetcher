// paperfetcher — LLM-agent-friendly CLI for academic paper search and retrieval
//
// 入口文件：解析 CLI 参数 → 加载配置 → 分发子命令 → 格式化输出

mod cli;
mod commands;
mod config;
mod error;
mod models;
mod output;
mod schema;
mod sources;
mod storage;

use clap::{CommandFactory, Parser};
use std::process::ExitCode;

use cli::{Cli, Commands};
use config::Config;
use error::AppError;
use output::{eprintln_info, print_error, print_success, OutputFormat};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let output_format: OutputFormat = cli.output.into();

    // 特殊命令：schema 和 completions 不需要配置
    match &cli.command {
        Commands::Schema { subcommand } => {
            return handle_schema(subcommand.as_deref());
        }
        Commands::Completions { shell } => {
            commands::completions::execute(*shell);
            return ExitCode::SUCCESS;
        }
        _ => {}
    }

    // 加载配置（合并默认值、配置文件、环境变量、CLI 参数）
    let config = match Config::load(
        cli.email.as_deref(),
        cli.config.as_deref(),
        cli.data_dir.as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            print_error(&e);
            return ExitCode::from(e.exit_code() as u8);
        }
    };

    if cli.verbose {
        eprintln_info(&format!("config loaded, data_dir={}", config.data_dir.display()));
    }

    // 分发子命令到对应的处理函数
    let result = dispatch_command(&cli.command, &config, cli.quiet).await;

    match result {
        Ok(output_data) => {
            print_success(&output_data, output_format);
            ExitCode::SUCCESS
        }
        Err(e) => {
            print_error(&e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// 分发子命令，返回统一的 JSON Value 作为输出
async fn dispatch_command(
    command: &Commands,
    config: &Config,
    quiet: bool,
) -> Result<serde_json::Value, AppError> {
    match command {
        Commands::Search {
            query,
            source,
            limit,
            year,
            open_access,
            sort: _,
        } => {
            let source_name = (*source).into();
            let resp = commands::search::execute(
                source_name,
                query,
                *limit,
                year.as_deref(),
                *open_access,
                config,
            )
            .await?;
            Ok(serde_json::to_value(resp)?)
        }

        Commands::Lookup { doi, source } => {
            let source_name = (*source).into();
            let resp = commands::lookup::execute(source_name, doi, config).await?;
            Ok(serde_json::to_value(resp)?)
        }

        Commands::Fetch {
            doi_or_file,
            output_dir: _,
            filename_template: _,
            overwrite,
            with_metadata,
            max_concurrent,
            timeout,
            source,
            stdin,
        } => {
            let source_names: Vec<sources::SourceName> =
                source.iter().map(|s| (*s).into()).collect();
            let resp = commands::fetch::execute(
                doi_or_file.as_deref(),
                *stdin,
                *overwrite,
                *with_metadata,
                *max_concurrent,
                *timeout,
                &source_names,
                config,
            )
            .await?;
            Ok(serde_json::to_value(resp)?)
        }

        Commands::List {
            filter,
            year,
            has_pdf: _,
            sort: _,
            limit,
        } => {
            let resp = commands::list::execute(
                filter.as_deref(),
                year.as_deref(),
                *limit,
                config,
            )?;
            Ok(serde_json::to_value(resp)?)
        }

        Commands::Status { doi } => {
            let resp = commands::status::execute(doi, config)?;
            Ok(serde_json::to_value(resp)?)
        }

        Commands::Remove { doi, force } => {
            // 非强制模式下输出确认提示
            if !force && !quiet {
                eprintln_info(&format!(
                    "removing local files for DOI: {doi} (use --force to skip confirmation)"
                ));
            }
            let resp = commands::remove::execute(doi, config)?;
            Ok(serde_json::to_value(resp)?)
        }

        // Schema 和 Completions 已在前面处理
        Commands::Schema { .. } | Commands::Completions { .. } => {
            unreachable!("handled before dispatch")
        }
    }
}

/// 处理 schema 子命令
fn handle_schema(subcommand: Option<&str>) -> ExitCode {
    let cmd = Cli::command();

    let schema = if let Some(sub_name) = subcommand {
        // 查找特定子命令
        if let Some(sub_cmd) = cmd.get_subcommands().find(|c| c.get_name() == sub_name) {
            schema::generate_schema(sub_cmd)
        } else {
            let err = AppError::NotFound {
                doi: format!("subcommand '{}'", sub_name),
            };
            print_error(&err);
            return ExitCode::from(err.exit_code() as u8);
        }
    } else {
        // 输出完整 schema
        schema::generate_schema(&cmd)
    };

    let json = serde_json::to_string_pretty(&schema).unwrap_or_default();
    println!("{json}");
    ExitCode::SUCCESS
}

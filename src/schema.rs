// Schema 生成 — 将 clap Command 树导出为 JSON，供 LLM 自省
use serde_json::{json, Value};

/// 递归遍历 clap Command 树，生成结构化 JSON schema
pub fn generate_schema(cmd: &clap::Command) -> Value {
    let mut schema = json!({
        "name": cmd.get_name(),
        "version": cmd.get_version().unwrap_or("unknown"),
        "description": cmd.get_about().map(|s| s.to_string()).unwrap_or_default(),
    });

    // 收集子命令
    let subcommands: Vec<Value> = cmd
        .get_subcommands()
        .filter(|sub| sub.get_name() != "help")
        .map(generate_subcommand_schema)
        .collect();

    if !subcommands.is_empty() {
        schema["subcommands"] = Value::Array(subcommands);
    }

    // 全局选项
    let global_args: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| a.is_global_set() || a.get_id() == "help" || a.get_id() == "version")
        .filter(|a| a.get_id() != "help" && a.get_id() != "version")
        .map(generate_arg_schema)
        .collect();

    if !global_args.is_empty() {
        schema["global_options"] = Value::Array(global_args);
    }

    // 使用示例
    schema["examples"] = json!([
        {
            "description": "Search for papers about transformers",
            "command": "paperfetcher search 'transformer attention mechanism' --limit 5"
        },
        {
            "description": "Lookup a specific paper by DOI",
            "command": "paperfetcher lookup '10.1038/s41586-021-03819-2'"
        },
        {
            "description": "Download a paper PDF",
            "command": "paperfetcher fetch '10.1038/s41586-021-03819-2' --with-metadata"
        },
        {
            "description": "List locally downloaded papers",
            "command": "paperfetcher list --limit 20"
        },
        {
            "description": "Check status of a downloaded paper",
            "command": "paperfetcher status '10.1038/s41586-021-03819-2'"
        },
        {
            "description": "Output as JSON Lines",
            "command": "paperfetcher --output jsonl search 'deep learning'"
        }
    ]);

    schema
}

/// 为单个子命令生成 schema
fn generate_subcommand_schema(cmd: &clap::Command) -> Value {
    let mut sub = json!({
        "name": cmd.get_name(),
        "description": cmd.get_about().map(|s| s.to_string()).unwrap_or_default(),
    });

    // 位置参数
    let positional: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| a.is_positional())
        .map(generate_arg_schema)
        .collect();

    if !positional.is_empty() {
        sub["positional_args"] = Value::Array(positional);
    }

    // 命名选项
    let options: Vec<Value> = cmd
        .get_arguments()
        .filter(|a| !a.is_positional() && a.get_id() != "help")
        .map(generate_arg_schema)
        .collect();

    if !options.is_empty() {
        sub["options"] = Value::Array(options);
    }

    sub
}

/// 为单个参数生成 schema
fn generate_arg_schema(arg: &clap::Arg) -> Value {
    let mut a = json!({
        "name": arg.get_id().as_str(),
    });

    if let Some(help) = arg.get_help() {
        a["description"] = json!(help.to_string());
    }

    // 长短标志
    if let Some(long) = arg.get_long() {
        a["long"] = json!(format!("--{long}"));
    }
    if let Some(short) = arg.get_short() {
        a["short"] = json!(format!("-{short}"));
    }

    // 是否必填
    a["required"] = json!(arg.is_required_set());

    // 默认值
    let defaults: Vec<&str> = arg.get_default_values().iter()
        .filter_map(|v| v.to_str())
        .collect();
    if !defaults.is_empty() {
        if defaults.len() == 1 {
            a["default"] = json!(defaults[0]);
        } else {
            a["default"] = json!(defaults);
        }
    }

    // 可选值列表
    let possible: Vec<String> = arg
        .get_possible_values()
        .into_iter()
        .map(|v| v.get_name().to_string())
        .collect();
    if !possible.is_empty() {
        a["possible_values"] = json!(possible);
    }

    // 环境变量
    if let Some(env) = arg.get_env() {
        a["env_var"] = json!(env.to_str().unwrap_or_default());
    }

    a
}

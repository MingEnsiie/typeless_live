mod ipc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use typeless_core::engine::{Engine, EngineConfig};
use typeless_llm::{GenOpts, LlmProvider, PostProcessor, PromptMode};
use typeless_storage::{models as model_registry, AppPaths, Db, Settings};

#[derive(Parser, Debug)]
#[command(name = "typeless-cli", version, about = "Typeless 语音输入法守护进程 / CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// 启动守护进程（监听全局快捷键）
    Run {
        /// 强制使用 mock provider（无需 API key）
        #[arg(long)]
        mock: bool,
        /// 单次模式：录制 N 秒后处理并退出（用于测试/headless）
        #[arg(long)]
        once: Option<u64>,
        /// 启动 Unix-socket IPC 服务器（供 fcitx5/ibus 集成）
        #[arg(long)]
        ipc: bool,
    },
    /// 配置管理
    Config {
        #[command(subcommand)]
        action: ConfigCmd,
    },
    /// 列出历史记录
    History {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// 词典管理
    Dict {
        #[command(subcommand)]
        action: DictCmd,
    },
    /// 一次性 pipeline：从文件读取 PCM 或文本，调用 LLM 后处理
    Refine {
        /// 输入文本（直接送 LLM）
        #[arg(long)]
        text: Option<String>,
        /// 使用 mock provider
        #[arg(long)]
        mock: bool,
    },
    /// 验收测试：跑通核心 pipeline，无需麦克风/网络
    Selftest,
    /// 模型管理（列出/下载/删除）
    Model {
        #[command(subcommand)]
        action: ModelCmd,
    },
    /// 交互式引导（首次使用）
    Onboarding,
    /// 流式 ASR 演示：录制 N 秒后边转写边输出 partial
    Stream {
        #[arg(long, default_value_t = 5)]
        secs: u64,
        #[arg(long)]
        mock: bool,
    },
    /// 测试 LLM 连通性
    PingLlm {
        #[arg(long)]
        mock: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ModelCmd {
    /// 列出可用模型
    Available,
    /// 列出已下载模型
    List,
    /// 下载模型
    Download { name: String },
    /// 删除模型
    Remove { name: String },
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// 显示当前配置
    Show,
    /// 设置：typeless-cli config set llm.api_key sk-xxx
    Set { key: String, value: String },
    /// 配置文件路径
    Path,
}

#[derive(Subcommand, Debug)]
enum DictCmd {
    List,
    Add { from: String, to: String, #[arg(long)] note: Option<String> },
    Remove { id: i64 },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::discover()?;

    // 文件日志 + stderr 日志（P1 #23）
    let _guard = init_logging(&paths);

    match cli.cmd {
        Cmd::Run { mock, once, ipc } => run_daemon(paths, mock, once, ipc).await,
        Cmd::Config { action } => handle_config(paths, action),
        Cmd::History { limit } => handle_history(paths, limit),
        Cmd::Dict { action } => handle_dict(paths, action),
        Cmd::Refine { text, mock } => handle_refine(paths, text, mock).await,
        Cmd::Selftest => selftest(paths).await,
        Cmd::Model { action } => handle_model(paths, action).await,
        Cmd::Onboarding => handle_onboarding(paths).await,
        Cmd::Stream { secs, mock } => handle_stream(paths, secs, mock).await,
        Cmd::PingLlm { mock } => handle_ping_llm(paths, mock).await,
    }
}

fn init_logging(paths: &AppPaths) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let env = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stderr);
    let log_dir = paths.log_dir.clone();
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(&log_dir, "typeless.log");
    let (nb, guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(nb);
    tracing_subscriber::registry()
        .with(env)
        .with(stderr_layer)
        .with(file_layer)
        .init();
    Some(guard)
}

fn build_provider(settings: &Settings, force_mock: bool) -> Arc<dyn LlmProvider> {
    if force_mock {
        return Arc::new(typeless_llm::MockLlm);
    }
    // 隐私模式：local_only 强制使用本地 HTTP provider 或 Mock（P1 #21）
    if settings.privacy.local_only {
        if settings.llm.provider == "local" || settings.llm.provider == "mock" {
            // fallthrough to normal switch
        } else {
            eprintln!("🔒 privacy.local_only=true，强制使用本地 LLM (LocalHttp)");
            return Arc::new(typeless_llm::LocalHttp::new(settings.llm.base_url.clone()));
        }
    }
    if settings.llm.provider == "local" {
        return Arc::new(typeless_llm::LocalHttp::new(settings.llm.base_url.clone()));
    }
    if settings.llm.provider == "mock" {
        return Arc::new(typeless_llm::MockLlm);
    }
    // 优先从 keyring 读取 api_key（P1 #20）
    let api_key = settings.llm.api_key.clone()
        .or_else(|| typeless_storage::secrets::get("llm_api_key"))
        .unwrap_or_default();
    if api_key.is_empty() {
        eprintln!("⚠ 未配置 LLM api_key，使用 Mock provider。设置：typeless-cli config set llm.api_key sk-xxx");
        return Arc::new(typeless_llm::MockLlm);
    }
    match settings.llm.provider.as_str() {
        "deepseek" => Arc::new(typeless_llm::DeepSeek::new(api_key, settings.llm.base_url.clone())),
        "mimo" => Arc::new(typeless_llm::MiMo::new(api_key, settings.llm.base_url.clone())),
        "openai" => {
            let base = settings.llm.base_url.clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".into());
            Arc::new(typeless_llm::OpenAiCompat::new("openai", base, api_key))
        }
        other => {
            eprintln!("⚠ 未知 provider {other}，回退 Mock");
            Arc::new(typeless_llm::MockLlm)
        }
    }
}

fn build_asr(settings: &Settings, paths: &AppPaths) -> Arc<dyn typeless_asr::AsrEngine> {
    #[cfg(feature = "whisper")]
    {
        if settings.asr.backend == "whisper" {
            let model_path = settings.asr.model_path.clone()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| paths.models_dir.join(&settings.asr.model));
            if model_path.exists() {
                match typeless_asr::WhisperAsr::load(model_path.clone()) {
                    Ok(w) => return Arc::new(w),
                    Err(e) => eprintln!("⚠ whisper 加载失败 ({e})，回退 mock"),
                }
            } else {
                eprintln!("⚠ Whisper 模型未找到: {} ，回退 mock。可使用 scripts/download-models.sh 下载", model_path.display());
            }
        }
    }
    let _ = (settings, paths);
    Arc::new(typeless_asr::MockAsr)
}

fn build_post(settings: &Settings, provider: Arc<dyn LlmProvider>, db: Option<&Db>) -> Arc<PostProcessor> {
    let mut opts = GenOpts::default();
    opts.model = settings.llm.model.clone();
    opts.temperature = settings.llm.temperature;
    opts.max_tokens = settings.llm.max_tokens;
    let mut p = PostProcessor::new(provider, opts);
    p.mode = PromptMode::parse(&settings.prompt_mode);
    p.language = settings.asr.language.clone();
    if let Some(db) = db {
        if let Ok(list) = db.dict_list() {
            p.dictionary = list.into_iter().map(|e| (e.from_text, e.to_text)).collect();
        }
    }
    let ctx = typeless_context::AppContext::detect();
    p.app_context = ctx.summary();
    Arc::new(p)
}

async fn run_daemon(paths: AppPaths, mock: bool, once: Option<u64>, ipc: bool) -> Result<()> {
    let settings = Settings::load_or_create(&paths.config_file)?;
    let db = Arc::new(Db::open(&paths.db_file)?);
    let provider = build_provider(&settings, mock);
    let asr = build_asr(&settings, &paths);
    let post = build_post(&settings, provider, Some(&db));
    let injector: Arc<dyn typeless_inject::TextInjector> = Arc::from(typeless_inject::default_injector());

    let cfg = EngineConfig {
        mode: typeless_audio::CaptureMode::Toggle,
        language: if settings.asr.language == "auto" { None } else { Some(settings.asr.language.clone()) },
        translate: settings.asr.translate,
        save_history: !settings.privacy.no_history,
        auto_inject: true,
    };
    let engine = Arc::new(Engine::new(asr, post, injector, Some(db), cfg));

    // 状态订阅打印
    let mut rx = engine.subscribe();
    tokio::spawn(async move {
        while let Ok(ev) = rx.recv().await {
            println!("[event] {}", serde_json::to_string(&ev).unwrap_or_default());
        }
    });

    // P2 #24/#25: 可选 IPC 服务器（供 fcitx5/ibus addon 接入）
    if ipc {
        let engine_ipc = engine.clone();
        tokio::spawn(async move {
            if let Err(e) = ipc::serve(engine_ipc).await {
                eprintln!("⚠ IPC 服务器停止: {e}");
            }
        });
        println!("🔌 IPC socket: {}", ipc::socket_path().display());
    }

    if let Some(secs) = once {
        // headless 单次模式
        engine.start_recording()?;
        println!("⏺  录音 {secs}s ...");
        tokio::time::sleep(std::time::Duration::from_secs(secs)).await;
        let out = engine.stop_and_process().await?;
        println!("✅ 输出: {:?}", out);
        return Ok(());
    }

    // 守护进程模式 + 全局热键
    let combo = settings.hotkey.trigger.clone();
    println!("🎙  Typeless 已启动。按下 {combo} 开始/结束录音。Ctrl+C 退出。");
    match typeless_hotkey::Hotkey::register(&combo) {
        Ok((_hk, rx_hk)) => {
            let engine2 = engine.clone();
            std::thread::spawn(move || {
                while let Ok(ev) = rx_hk.recv() {
                    if let typeless_hotkey::HotkeyEvent::Press = ev {
                        let e = engine2.clone();
                        tokio::runtime::Handle::current().spawn(async move {
                            if e.is_recording() {
                                let _ = e.stop_and_process().await;
                            } else {
                                let _ = e.start_recording();
                            }
                        });
                    }
                }
            });
        }
        Err(e) => {
            eprintln!("⚠ 注册全局热键失败: {e}\n   可能是无显示服务器；改用 stdin 触发：按回车键 toggle 录音。");
            let engine2 = engine.clone();
            tokio::spawn(async move {
                let stdin = tokio::io::AsyncBufReadExt::lines(
                    tokio::io::BufReader::new(tokio::io::stdin())
                );
                tokio::pin!(stdin);
                while let Ok(Some(_)) = stdin.next_line().await {
                    if engine2.is_recording() {
                        let _ = engine2.stop_and_process().await;
                    } else {
                        let _ = engine2.start_recording();
                    }
                }
            });
        }
    }

    tokio::signal::ctrl_c().await?;
    println!("\n👋 退出");
    Ok(())
}

fn handle_config(paths: AppPaths, action: ConfigCmd) -> Result<()> {
    match action {
        ConfigCmd::Path => {
            println!("{}", paths.config_file.display());
        }
        ConfigCmd::Show => {
            let s = Settings::load_or_create(&paths.config_file)?;
            println!("{}", toml::to_string_pretty(&s)?);
        }
        ConfigCmd::Set { key, value } => {
            let mut s = Settings::load_or_create(&paths.config_file)?;
            s.set_dotted(&key, &value)?;
            // api_key 同时存 keyring
            if key == "llm.api_key" {
                if let Err(e) = typeless_storage::secrets::set("llm_api_key", &value) {
                    eprintln!("⚠ keyring 写入失败: {e}");
                }
            }
            s.save(&paths.config_file)?;
            println!("✅ 已设置 {key} = ***");
        }
    }
    Ok(())
}

fn handle_history(paths: AppPaths, limit: usize) -> Result<()> {
    let db = Db::open(&paths.db_file)?;
    let list = db.list_history(limit)?;
    for r in list {
        println!("{}  [{}]  {} → {}",
                 r.created_at.format("%Y-%m-%d %H:%M:%S"),
                 r.mode,
                 truncate(&r.raw_text, 30),
                 truncate(&r.final_text, 60));
    }
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n { s.to_string() } else {
        format!("{}…", chars[..n].iter().collect::<String>())
    }
}

fn handle_dict(paths: AppPaths, action: DictCmd) -> Result<()> {
    let db = Db::open(&paths.db_file)?;
    match action {
        DictCmd::List => {
            for e in db.dict_list()? {
                println!("#{}  {} → {}  {}", e.id, e.from_text, e.to_text, e.note.unwrap_or_default());
            }
        }
        DictCmd::Add { from, to, note } => {
            db.dict_upsert(&from, &to, note.as_deref())?;
            println!("✅ 已添加");
        }
        DictCmd::Remove { id } => {
            db.dict_delete(id)?;
            println!("✅ 已删除");
        }
    }
    Ok(())
}

async fn handle_refine(paths: AppPaths, text: Option<String>, mock: bool) -> Result<()> {
    let settings = Settings::load_or_create(&paths.config_file)?;
    let db = Db::open(&paths.db_file)?;
    let provider = build_provider(&settings, mock);
    let post = build_post(&settings, provider, Some(&db));
    let input = text.unwrap_or_else(|| "嗯 啊 那个 你好啊 这个 typeless 嗯 真的很好用".to_string());
    println!("RAW : {input}");
    let out = post.refine(&input).await?;
    println!("FINAL: {out}");
    Ok(())
}

async fn handle_model(paths: AppPaths, action: ModelCmd) -> Result<()> {
    match action {
        ModelCmd::Available => {
            for m in model_registry::registry() {
                println!("[{}] {:<22} {:>5}MB  {}", m.kind, m.name, m.size_mb, m.description);
            }
        }
        ModelCmd::List => {
            let dir = &paths.models_dir;
            if !dir.exists() {
                println!("(模型目录不存在: {})", dir.display());
                return Ok(());
            }
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let meta = entry.metadata()?;
                if meta.is_file() {
                    let mb = meta.len() / 1_048_576;
                    println!("{:<32} {:>5}MB", entry.file_name().to_string_lossy(), mb);
                }
            }
        }
        ModelCmd::Download { name } => {
            let desc = model_registry::find(&name)
                .ok_or_else(|| anyhow::anyhow!("未知模型: {name}（用 `model available` 查看）"))?;
            let dest = paths.models_dir.join(&desc.name);
            if dest.exists() {
                println!("✅ 已存在: {}", dest.display());
                return Ok(());
            }
            std::fs::create_dir_all(&paths.models_dir)?;
            println!("⬇  下载 {} ({}MB) → {}", desc.url, desc.size_mb, dest.display());
            let resp = reqwest::get(&desc.url).await?;
            let total = resp.content_length().unwrap_or(0);
            let mut stream = resp.bytes_stream();
            use futures::StreamExt;
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::File::create(&dest).await?;
            let mut downloaded = 0u64;
            let mut last_pct = 0u64;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                file.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;
                if total > 0 {
                    let pct = downloaded * 100 / total;
                    if pct >= last_pct + 5 {
                        last_pct = pct;
                        eprintln!("  {pct}%  ({}/{} MB)", downloaded / 1_048_576, total / 1_048_576);
                    }
                }
            }
            file.flush().await?;
            println!("✅ 完成");
        }
        ModelCmd::Remove { name } => {
            let p = paths.models_dir.join(&name);
            if p.exists() {
                std::fs::remove_file(&p)?;
                println!("✅ 已删除 {}", p.display());
            } else {
                eprintln!("⚠ 文件不存在: {}", p.display());
            }
        }
    }
    Ok(())
}

async fn handle_onboarding(paths: AppPaths) -> Result<()> {
    use std::io::{BufRead, Write};
    let mut s = Settings::load_or_create(&paths.config_file)?;

    let prompt = |msg: &str, default: &str| -> Result<String> {
        print!("{msg} [{default}]: ");
        std::io::stdout().flush()?;
        let mut line = String::new();
        std::io::stdin().lock().read_line(&mut line)?;
        let v = line.trim().to_string();
        Ok(if v.is_empty() { default.to_string() } else { v })
    };

    println!("🎙  欢迎使用 Typeless！");
    println!("配置目录: {}", paths.config_dir.display());

    let provider = prompt("LLM provider (deepseek/mimo/openai/local/mock)", &s.llm.provider)?;
    s.llm.provider = provider.clone();
    if matches!(provider.as_str(), "deepseek" | "mimo" | "openai") {
        print!("{} api_key (留空跳过): ", provider);
        std::io::stdout().flush()?;
        let mut k = String::new();
        std::io::stdin().lock().read_line(&mut k)?;
        let k = k.trim();
        if !k.is_empty() {
            s.llm.api_key = Some(k.to_string());
            let _ = typeless_storage::secrets::set("llm_api_key", k);
        }
    } else if provider == "local" {
        let url = prompt("本地服务 base_url", "http://localhost:8080/v1")?;
        s.llm.base_url = Some(url);
    }

    s.hotkey.trigger = prompt("全局热键", &s.hotkey.trigger)?;
    s.asr.language = prompt("ASR 语言 (auto/zh/en/ja)", &s.asr.language)?;
    s.asr.backend = prompt("ASR backend (mock/whisper)", &s.asr.backend)?;

    if s.asr.backend == "whisper" {
        let want = prompt("下载 whisper 模型？(ggml-base.bin/no)", "ggml-base.bin")?;
        if want != "no" && !want.is_empty() {
            let dest = paths.models_dir.join(&want);
            if !dest.exists() {
                println!("→ 运行: typeless-cli model download {want}");
            } else {
                s.asr.model = want;
            }
        }
    }

    s.save(&paths.config_file)?;
    println!("✅ 配置已保存到 {}", paths.config_file.display());
    println!("下一步：typeless-cli run");
    Ok(())
}

async fn handle_stream(paths: AppPaths, secs: u64, mock: bool) -> Result<()> {
    use std::sync::Arc;
    let settings = Settings::load_or_create(&paths.config_file)?;
    let asr = if mock {
        Arc::new(typeless_asr::MockAsr) as Arc<dyn typeless_asr::AsrEngine>
    } else {
        build_asr(&settings, &paths)
    };
    let stream = Arc::new(typeless_asr::streaming::StreamingAsr::new(asr));
    let (pcm_tx, pcm_rx) = tokio::sync::mpsc::channel::<Vec<i16>>(32);
    let mut partial_rx = stream.spawn(pcm_rx, Default::default());

    // 模拟 PCM 输入：每 200ms 推一段静音（mock 才有意义）
    let total_chunks = (secs * 5) as usize;
    tokio::spawn(async move {
        for _ in 0..total_chunks {
            let chunk = vec![0i16; 16_000 / 5];
            if pcm_tx.send(chunk).await.is_err() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    });

    println!("⏺  流式转写 {secs}s ...");
    while let Some(text) = partial_rx.recv().await {
        println!("[partial] {text}");
    }
    println!("✅ 完成");
    Ok(())
}

async fn handle_ping_llm(paths: AppPaths, mock: bool) -> Result<()> {
    let settings = Settings::load_or_create(&paths.config_file)?;
    let provider = build_provider(&settings, mock);
    println!("→ provider: {}", provider.name());
    let mut opts = GenOpts::default();
    opts.model = settings.llm.model.clone();
    opts.max_tokens = 32;
    let msgs = vec![
        typeless_llm::Message::system("Reply with the single word: pong"),
        typeless_llm::Message::user("ping"),
    ];
    let t0 = std::time::Instant::now();
    match provider.complete(msgs, &opts).await {
        Ok(out) => {
            println!("✅ {}ms  → {}", t0.elapsed().as_millis(), out.trim());
        }
        Err(e) => {
            eprintln!("❌ {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

async fn selftest(paths: AppPaths) -> Result<()> {
    println!("🧪 Typeless Selftest");
    println!("paths: {:?}", paths.config_dir);
    let settings = Settings::load_or_create(&paths.config_file)?;
    println!("✓ settings loaded ({} provider)", settings.llm.provider);
    let db = Db::open(&paths.db_file)?;
    println!("✓ database opened");
    let provider: Arc<dyn LlmProvider> = Arc::new(typeless_llm::MockLlm);
    let asr: Arc<dyn typeless_asr::AsrEngine> = Arc::new(typeless_asr::MockAsr);
    let post = build_post(&settings, provider, Some(&db));
    let injector: Arc<dyn typeless_inject::TextInjector> =
        Arc::new(typeless_inject::clipboard::ClipboardInjector);

    // 模拟 1.5s PCM
    let pcm: Vec<i16> = vec![0; 16_000 * 3 / 2];
    let asr_res = asr.transcribe(&pcm, &Default::default()).await?;
    println!("✓ ASR mock → {}", asr_res.text);
    let refined = post.refine(&asr_res.text).await?;
    println!("✓ LLM mock refine → {}", refined);
    // 不真注入（避免影响系统剪贴板的 inject 函数仍接受 spawn_blocking 失败）
    let _ = injector;
    println!("✅ Selftest passed.");
    Ok(())
}

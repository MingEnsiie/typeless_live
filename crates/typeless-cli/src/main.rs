use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use typeless_core::engine::{Engine, EngineConfig};
use typeless_llm::{GenOpts, LlmProvider, PostProcessor, PromptMode};
use typeless_storage::{AppPaths, Db, Settings};

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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let paths = AppPaths::discover()?;

    match cli.cmd {
        Cmd::Run { mock, once } => run_daemon(paths, mock, once).await,
        Cmd::Config { action } => handle_config(paths, action),
        Cmd::History { limit } => handle_history(paths, limit),
        Cmd::Dict { action } => handle_dict(paths, action),
        Cmd::Refine { text, mock } => handle_refine(paths, text, mock).await,
        Cmd::Selftest => selftest(paths).await,
    }
}

fn build_provider(settings: &Settings, force_mock: bool) -> Arc<dyn LlmProvider> {
    if force_mock || settings.privacy.local_only && settings.llm.api_key.is_none() {
        return Arc::new(typeless_llm::MockLlm);
    }
    let api_key = settings.llm.api_key.clone().unwrap_or_default();
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
    if let Some(db) = db {
        if let Ok(list) = db.dict_list() {
            p.dictionary = list.into_iter().map(|e| (e.from_text, e.to_text)).collect();
        }
    }
    let ctx = typeless_context::AppContext::detect();
    p.app_context = ctx.summary();
    Arc::new(p)
}

async fn run_daemon(paths: AppPaths, mock: bool, once: Option<u64>) -> Result<()> {
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

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use getrandom::fill as random_fill;
use lifegraph_agent::{AgentConfig, AgentEndpoint, LifegraphAgent, PutBlobRequest};

const DEFAULT_DATA_ROOT: &str = "/data/lifegraph-agent";
const DEFAULT_NAMESPACE: &str = "lifegraph";
const DEFAULT_DATABASE: &str = "core";
const BOOTSTRAP_KEY_FILE: &str = "bootstrap-test-key.hex";
const HEARTBEAT_PERIOD: Duration = Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_root =
        env_path("LIFEGRAPH_AGENT_DATA_ROOT").unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_ROOT));
    fs::create_dir_all(&data_root)?;

    let key_path = data_root.join(BOOTSTRAP_KEY_FILE);
    let bootstrap_key_hex = load_or_create_bootstrap_key(&key_path)?;
    let agent_id = env::var("LIFEGRAPH_AGENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("test-{}", &bootstrap_key_hex[..16]));

    let namespace =
        env::var("LIFEGRAPH_AGENT_NAMESPACE").unwrap_or_else(|_| DEFAULT_NAMESPACE.to_owned());
    let database =
        env::var("LIFEGRAPH_AGENT_DATABASE").unwrap_or_else(|_| DEFAULT_DATABASE.to_owned());
    let db_path = data_root.join("surreal");
    let blob_root = data_root.join("blobs");
    fs::create_dir_all(&db_path)?;
    fs::create_dir_all(&blob_root)?;

    let agent = LifegraphAgent::connect(AgentConfig {
        agent_id: agent_id.clone(),
        namespace,
        database,
        endpoint: AgentEndpoint::SurrealKv { path: db_path },
        blob_root: blob_root.clone(),
    })
    .await?;

    println!(
        "lifegraph-agent started: agent_id={} data_root={} key_path={}",
        agent_id,
        data_root.display(),
        key_path.display()
    );

    ensure_bootstrap_blob(&agent).await?;
    agent.heartbeat(now_unix_ms()).await?;

    let mut ticker = tokio::time::interval(HEARTBEAT_PERIOD);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                agent.heartbeat(now_unix_ms()).await?;
                println!("lifegraph-agent heartbeat agent_id={}", agent.config().agent_id);
            }
            result = tokio::signal::ctrl_c() => {
                result?;
                println!("lifegraph-agent shutting down agent_id={}", agent.config().agent_id);
                break;
            }
        }
    }

    Ok(())
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}

fn load_or_create_bootstrap_key(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if path.exists() {
        let mut text = String::new();
        OpenOptions::new()
            .read(true)
            .open(path)?
            .read_to_string(&mut text)?;
        let text = text.trim().to_owned();
        if !text.is_empty() {
            return Ok(text);
        }
    }

    let mut key = [0u8; 32];
    random_fill(&mut key)?;
    let hex_key = hex::encode(key);

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)?;
    file.write_all(hex_key.as_bytes())?;
    file.write_all(b"\n")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(hex_key)
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

async fn ensure_bootstrap_blob(agent: &LifegraphAgent) -> Result<(), Box<dyn std::error::Error>> {
    let mut ciphertext = [0u8; 48];
    random_fill(&mut ciphertext)?;
    let blob_id = format!("bootstrap-{}", agent.config().agent_id);
    let meta = agent
        .put_blob(&PutBlobRequest {
            blob_id,
            ciphertext: ciphertext.to_vec(),
            encryption_suite: "testing.random-ciphertext.v1".to_owned(),
            recipients: vec![agent.config().agent_id.clone()],
        })
        .await?;
    println!(
        "lifegraph-agent stored bootstrap ciphertext blob={} path={} size={}",
        meta.blob_id,
        agent.config().blob_root.join(meta.relative_path).display(),
        meta.size_bytes
    );
    Ok(())
}

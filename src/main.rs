use std::sync::Arc;
use base64::Engine;
use tokio::sync::Mutex;

mod rocket;
mod git;
mod modals;

#[derive(Debug, Copy, Clone)]
pub struct Limits {
    max_permission_level: u64,
    max_manage_permission_level: u64,
}

const NEOLUMA_GIT_REPO_REMOTE_URL:&str = "git@github.com:The-Land-of-Future/TLOF-Club.git";
#[derive(Copy, Clone)]
pub struct Keypair {
    pub public: [u8; sphincsplus::CRYPTO_PUBLICKEYBYTES as usize],
    pub secret: [u8; sphincsplus::CRYPTO_SECRETKEYBYTES as usize],
}

pub(crate) async fn get_db<'a>() -> sqlx::postgres::PgPool {
    static DB: tokio::sync::OnceCell<sqlx::postgres::PgPool> = tokio::sync::OnceCell::const_new();
    DB.get_or_init(||async {
        let options = sqlx::postgres::PgConnectOptions::new();
        let pool = sqlx::Pool::connect_with(options).await.expect("Failed to connect to postgres");
        tracing::info!("Connected to postgres");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to migrate");
        pool
    }).await.clone()
}

fn main() -> ::anyhow::Result<()> {
    if let Err(err) = dotenvy::dotenv() {
        eprintln!("Dotenv does not exist at path: '{}' ?", err);
    }
    {
        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .pretty()
                    .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
            )
            .init();
        tracing::info!("Initialized Logging.")
    }

    let repo_path_str = std::env::var("NEOLUMA_GIT_REPO_DIR")?;
    let repo_path = std::path::Path::new(&repo_path_str);
    let repo = match repo_path.metadata() {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let mut fetch_opts = git2::FetchOptions::new();
            let mut callbacks = git2::RemoteCallbacks::new();
            callbacks.transfer_progress(|progress|{
                tracing::info!(
                    "deltas: {}/{}, objects: {}/{}, received bytes: {}, received objects: {}, local objects: {}",
                    progress.indexed_deltas(), progress.total_deltas(),
                    progress.indexed_objects(), progress.total_objects(),
                    progress.received_bytes(), progress.received_objects(),
                    progress.local_objects()
                );
                true
            });
            callbacks.sideband_progress(|progress|{
                let progress = match core::str::from_utf8(progress) {
                    Ok(progress) => progress,
                    Err(err) => {
                        tracing::info!("Received progress from remote, but the progress isn't utf-8: {err}");
                        return true;
                    }
                };
                tracing::info!("Remote: {progress}");
                true
            });
            git::auth::add_auth(&mut callbacks).map_err(|v|::anyhow::format_err!("{v}"))?;
            fetch_opts.remote_callbacks(callbacks);
            git2::build::RepoBuilder::new()
                .fetch_options(fetch_opts)
                .clone(NEOLUMA_GIT_REPO_REMOTE_URL, repo_path)
                .map_err(|err|anyhow::format_err!("Could not clone NeoLuma Git Repo: {err}"))?
        }
        Err(err) => {
            anyhow::bail!("Cannot query metadata of '{repo_path_str}': {err}")
        }
        Ok(v) if !v.file_type().is_dir() => {
            anyhow::bail!("'{repo_path_str}' is not a directory")
        }
        Ok(_) => {
            git2::Repository::open(repo_path)
                .map_err(|err|anyhow::format_err!("Could not discover repo at path '{repo_path_str}': {err}"))?
        }
    };

    let pk = std::env::var("NEOLUMA_SPHINCSPLUS_PK").map_err(|err|::anyhow::format_err!("Could not find NEOLUMA_SPHINCSPLUS_PK: {err}"))?;
    let sk = std::env::var("NEOLUMA_SPHINCSPLUS_SK").map_err(|err|::anyhow::format_err!("Could not find NEOLUMA_SPHINCSPLUS_SK: {err}"))?;
    let pk = base64::engine::general_purpose::STANDARD.decode(pk.as_bytes()).map_err(|err|::anyhow::format_err!("Could not base64 decode NEOLUMA_SPHINCSPLUS_PK: {err}"))?;
    let sk = base64::engine::general_purpose::STANDARD.decode(sk.as_bytes()).map_err(|err|::anyhow::format_err!("Could not base64 decode NEOLUMA_SPHINCSPLUS_SK: {err}"))?;
    let pk = pk.as_chunks::<{sphincsplus::CRYPTO_PUBLICKEYBYTES as usize}>();
    let sk = sk.as_chunks::<{sphincsplus::CRYPTO_SECRETKEYBYTES as usize}>();
    if pk.1.len() != 0 || sk.0.len() != 1 { anyhow::bail!("Public key should be {} bytes long", sphincsplus::CRYPTO_PUBLICKEYBYTES) }
    if sk.1.len() != 0 || sk.0.len() != 1 { anyhow::bail!("Secret key should be {} bytes long", sphincsplus::CRYPTO_SECRETKEYBYTES) }
    let mk = Keypair {
        public: pk.0[0],
        secret: sk.0[0],
    };

    ::rocket::execute(main_async(repo, mk))
}

async fn main_async(repo: git2::Repository, mk: Keypair) -> ::anyhow::Result<()> {
    let _ = get_db().await;
    let (client, discord) = rocket::auth::discord::setup().await?;
    let client_shard_manager = client.shard_manager.clone();
    let client_jh = ::tokio::spawn(async{
        let mut client = client;
        tracing::info!("Starting Discord Shards");
        client.start_autosharded().await
    });

    let limits = Limits {
        max_permission_level:
            std::env::var("NEOLUMA_MAX_PERMISSION_LEVEL")
                .map_err(|err|::anyhow::format_err!("Could not find NEOLUMA_MAX_PERMISSION_LEVEL: {err}"))?
                .parse::<u64>()
                .map_err(|err|::anyhow::format_err!("Could not parse NEOLUMA_MAX_PERMISSION_LEVEL as u64: {err}"))?
                .min(i16::MAX as u64)
        ,
        max_manage_permission_level: i32::MAX as u64,
    };

    let rocket  = ::rocket::Rocket::build()
        .mount("/", ::rocket::routes![
            rocket::auth::discord::new::new_oauth,
            rocket::auth::discord::oauth::oauth_ok,
            rocket::auth::discord::err::oauth_err,

            rocket::api::club::code_replacements::put_club_replacement,
            rocket::api::club::code_replacements::delete_club_replacement,
            rocket::api::club::vrcuser_level::put_vrcuser_level,
            rocket::api::club::vrcuser_level::delete_vrcuser_level,
            rocket::api::club::club_name::put_club_name,
            rocket::api::club::manage_permissions::put_club_permission,
            rocket::api::club::manage_permissions::new_club_permission,
            rocket::api::club::manage_permissions::delete_club_permission,
            rocket::api::club::publish::post_publish,
            rocket::api::club::image::put_image,
            rocket::api::club::image::get_image,

            rocket::club::get_club,
            rocket::club::instance::get_club_instance,
            rocket::club::vrchat_permissions::get_club_vrc_names,
            rocket::club::manage_permissions::get_club_discord_permissions,
        ])
        .manage(Arc::new(Mutex::new(repo)))
        .manage(mk)
        .manage(discord)
        .manage(limits)
    ;

    tracing::info!("Igniting Rocket");
    let rocket = rocket.ignite().await?;
    tracing::info!("Ignited Rocket. About to Launch!");
    rocket.launch().await?;
    tracing::info!("Launched Rocket, Shutdown Now in progress");

    client_shard_manager.shutdown_all().await;
    tracing::info!("Requested shutdown of all Discord Shards");
    client_jh.await??;
    tracing::info!("Shutdown Discord Client");

    Ok(())
}

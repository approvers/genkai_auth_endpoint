#![feature(try_blocks)]

use std::convert::Infallible;

use anyhow::{Context as _, Error, Result};
use mongodb::{bson::doc, options::ClientOptions, Client, Collection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use warp::http::StatusCode;
use warp::reply::with_status;
use warp::{Filter, Rejection, Reply};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenkaiAuthData {
    user_id: String,
    pgp_pub_key: Option<String>,
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let use_ansi = env_var("NO_COLOR").is_err();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(use_ansi)
        .init();

    let opt = ClientOptions::parse(&env_var("MONGO_AUTH_DB_URI")?)
        .await
        .context("failed to parse mongodb uri")?;

    let db = Client::with_options(opt)
        .context("failed to create mongodb client")?
        .database("RustyPonyo")
        .collection::<GenkaiAuthData>("GenkaiAuth");

    let route = warp::path!("v1" / "auth")
        .and(warp::post())
        .and(warp::header("Authorization"))
        .and(inject(db))
        .and_then(handle)
        .recover(recover)
        .with(warp::trace::request());

    let port = env_var("PORT")
        .ok()
        .map(|x| x.parse().context("failed to parse PORT env var"))
        .transpose()?
        .unwrap_or(3000);

    warp::serve(route).bind(([0, 0, 0, 0], port)).await;

    Ok(())
}

async fn handle(
    auth_header: String,
    db: Collection<GenkaiAuthData>,
) -> Result<impl Reply, Rejection> {
    let status: Result<_, _> = try {
        let token = match auth_header.strip_prefix("Bearer ") {
            Some(t) => t,
            None => {
                return Ok(with_status(
                    error_json("Authorization header must begin with \"Bearer\""),
                    StatusCode::BAD_REQUEST,
                ))
            }
        };

        let mut hasher = Sha512::new();
        hasher.update(token.trim());
        let token = hasher.finalize();
        let token = hex::encode(token);

        let entry = match db
            .find_one(doc! { "token": token }, None)
            .await
            .context("failed to find")?
        {
            Some(u) => u,
            None => {
                return Ok(with_status(
                    error_json("Invalid token"),
                    StatusCode::UNAUTHORIZED,
                ))
            }
        };

        with_status(
            format!(r#"{{"user_id":"{}"}}"#, entry.user_id),
            StatusCode::OK,
        )
    };

    status.map_err(|e| warp::reject::custom(InternalError(e)))
}

#[derive(Debug)]
struct InternalError(Error);
impl warp::reject::Reject for InternalError {}
impl From<Error> for InternalError {
    fn from(e: Error) -> Self {
        InternalError(e)
    }
}

async fn recover(e: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(InternalError(e)) = e.find() {
        tracing::error!("{:#?}", e);
        return Ok(with_status(
            error_json("Internal Server Error"),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    Err(e)
}

fn inject<T: Send + Sync + Clone>(v: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || v.clone())
}

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("failed to get {} environment variable", name))
}

fn error_json(text: &str) -> String {
    format!(r#"{{"error":"{}"}}"#, text)
}

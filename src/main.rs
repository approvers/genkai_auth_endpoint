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

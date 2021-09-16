
fn inject<T: Send + Sync + Clone>(v: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone {
    warp::any().map(move || v.clone())
}

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("failed to get {} environment variable", name))
}

fn error_json(text: &str) -> String {
    format!(r#"{{"error":"{}"}}"#, text)
}

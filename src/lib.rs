use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let url = req.url()?;

    // Static Assets serves generated HTML and files before this Worker runs. This
    // fallback keeps non-browser requests and explicit misses under Rust control.
    env.assets("ASSETS")?.fetch(url.to_string(), None).await
}

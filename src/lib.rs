use worker::*;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // Static Assets serves generated HTML and files before this Worker runs. This
    // fallback preserves the original method, headers, and request semantics for
    // misses and non-browser requests.
    env.assets("ASSETS")?.fetch_request(req).await
}

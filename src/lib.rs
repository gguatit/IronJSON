pub mod config;
pub mod engine;
pub mod error;
pub mod handler;
pub mod rule;

use worker::{event, Context, Env, Request, Response, Result};

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();
    handler::handle_request(req, env).await
}

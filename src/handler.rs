use crate::engine::JsonEngine;
use crate::error::IronError;
use crate::rule::Direction;
use worker::*;

pub async fn handle_request(req: Request, env: Env) -> Result<Response> {
    let url = req.url()?;
    let path = url.path();
    let method = req.method().to_string();

    let rules_json: Option<String> = env
        .var("IRONJSON_RULES")
        .ok()
        .map(|v| v.to_string());

    let engine = JsonEngine::new(rules_json.as_deref())
        .map_err(|e| -> worker::Error { e.to_string().into() })?;

    match (method.as_str(), path) {
        ("POST", "/process") => handle_process(req, &engine).await,
        ("GET", "/health") => Response::from_json(&serde_json::json!({
            "status": "healthy",
            "service": "ironjson"
        })),
        ("GET", "/") | ("GET", "") => Response::from_json(&serde_json::json!({
            "service": "IronJSON - Edge JSON Accelerator",
            "endpoints": {
                "POST /process": "Direct JSON processing with rule application",
                "GET /health": "Health check"
            },
            "headers": {
                "x-ironjson-path": "Target API path for rule matching (default: /api/*)",
                "x-ironjson-direction": "request | response | both (default: request)"
            }
        })),
        _ => Response::error("Not Found", 404),
    }
}

async fn handle_process(mut req: Request, engine: &JsonEngine) -> Result<Response> {
    let direction = req
        .headers()
        .get("x-ironjson-direction")
        .ok()
        .flatten()
        .map(|d| match d.to_lowercase().as_str() {
            "response" => Direction::Response,
            "both" => Direction::Both,
            _ => Direction::Request,
        })
        .unwrap_or(Direction::Request);

    let target_path = req
        .headers()
        .get("x-ironjson-path")
        .ok()
        .flatten()
        .unwrap_or_else(|| "/api/*".to_string());

    let body_text = req.text().await?;

    match engine.process(&target_path, "POST", direction, body_text.as_bytes()) {
        Ok(result) => {
            let response = serde_json::json!({
                "success": true,
                "data": result
            });
            Response::from_json(&response)
        }
        Err(e) => build_error_response(&e),
    }
}

fn build_error_response(e: &IronError) -> Result<Response> {
    let status = e.http_status();
    let body = e.to_response_json();
    Response::error(serde_json::to_string(&body).unwrap_or_default(), status)
}

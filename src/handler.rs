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
        ("GET", "/") | ("GET", "") => {
            let mut resp = Response::from_html(LANDING_HTML)?;
            resp.headers_mut().set("content-type", "text/html;charset=utf-8")?;
            Ok(resp)
        }
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

static LANDING_HTML: &str = r##"<!DOCTYPE html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>IronJSON — Edge JSON Accelerator</title>
<style>
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:'Segoe UI',system-ui,-apple-system,sans-serif;background:#0a0e17;color:#e2e8f0;line-height:1.7}
a{color:#60a5fa;text-decoration:none}
a:hover{text-decoration:underline}
.wrap{max-width:960px;margin:0 auto;padding:40px 24px}
header{text-align:center;padding:60px 0 40px}
header h1{font-size:3rem;font-weight:800;background:linear-gradient(135deg,#60a5fa,#a78bfa);-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:8px}
header p{font-size:1.15rem;color:#94a3b8;max-width:600px;margin:0 auto}
.badge{display:inline-block;background:#1e293b;border:1px solid #334155;border-radius:20px;padding:4px 14px;font-size:.8rem;color:#94a3b8;margin:12px 4px}
.badge b{color:#a78bfa}
section{margin:48px 0}
section h2{font-size:1.5rem;font-weight:700;margin-bottom:16px;color:#f1f5f9;border-left:3px solid #60a5fa;padding-left:12px}
.card{background:#111827;border:1px solid #1e293b;border-radius:12px;padding:24px;margin-bottom:16px}
.card h3{font-size:1.1rem;color:#60a5fa;margin-bottom:8px}
.card p,.card li{color:#94a3b8;font-size:.92rem}
.card ul{padding-left:20px;margin-top:6px}
.card li{margin-bottom:4px}
.flow{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin:20px 0}
.flow .step{background:#1e293b;border:1px solid #334155;border-radius:8px;padding:8px 16px;font-size:.85rem;color:#e2e8f0;text-align:center;flex:1;min-width:100px}
.flow .arrow{color:#475569;font-size:1.2rem;flex:0}
pre{background:#0f172a;border:1px solid #1e293b;border-radius:8px;padding:16px;overflow-x:auto;font-size:.85rem;line-height:1.6;color:#e2e8f0;margin:12px 0}
pre .k{color:#7dd3fc}pre .s{color:#86efac}pre .n{color:#fbbf24}pre .c{color:#64748b}
.tabs{display:flex;gap:4px;margin-bottom:0}
.tab{background:#1e293b;border:1px solid #334155;border-radius:8px 8px 0 0;padding:8px 18px;font-size:.85rem;color:#94a3b8;cursor:pointer}
.tab.active{background:#111827;color:#f1f5f9;border-bottom-color:#111827}
.tab-panel{display:none}.tab-panel.active{display:block}
.try{background:#111827;border:1px solid #1e293b;border-radius:12px;padding:24px;margin-top:24px}
.try textarea{width:100%;min-height:120px;background:#0f172a;border:1px solid #334155;border-radius:8px;color:#e2e8f0;font-family:'Cascadia Code',Consolas,monospace;font-size:.85rem;padding:12px;resize:vertical}
.try .row{display:flex;gap:12px;margin-top:12px;flex-wrap:wrap}
.try select,.try button{padding:10px 18px;border-radius:8px;border:1px solid #334155;font-size:.9rem}
.try select{background:#1e293b;color:#e2e8f0;flex:1;min-width:140px}
.try button{background:linear-gradient(135deg,#3b82f6,#8b5cf6);color:#fff;border:none;cursor:pointer;font-weight:600;min-width:140px;transition:opacity .2s}
.try button:hover{opacity:.85}
.try button:disabled{opacity:.5;cursor:not-allowed}
#result{margin-top:16px;min-height:60px}
footer{text-align:center;color:#475569;font-size:.8rem;padding:40px 0 24px;border-top:1px solid #1e293b;margin-top:48px}
</style>
</head>
<body>
<div class="wrap">

<header>
<h1>IronJSON</h1>
<p>Cloudflare Workers에서 동작하는 Rust 기반 초고속 JSON 처리 엔진</p>
<div>
<span class="badge"><b>Rust</b> + WebAssembly</span>
<span class="badge"><b>Edge</b> Computing</span>
<span class="badge"><b>Zero</b>-Copy Pipeline</span>
</div>
</header>

<section>
<h2>작동 방식</h2>
<div class="flow">
<div class="step">JSON 수신</div><div class="arrow">&rarr;</div>
<div class="step">Rule 매칭</div><div class="arrow">&rarr;</div>
<div class="step">스키마 검증</div><div class="arrow">&rarr;</div>
<div class="step">필드 제거</div><div class="arrow">&rarr;</div>
<div class="step">마스킹</div><div class="arrow">&rarr;</div>
<div class="step">구조 변환</div><div class="arrow">&rarr;</div>
<div class="step">JSON 응답</div>
</div>
</section>

<section>
<h2>핵심 기능</h2>
<div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:16px">
<div class="card">
<h3>스키마 검증</h3>
<p>type, required, min/max, min_length, pattern 등 경량 JSON Schema로 요청을 검증합니다.</p>
</div>
<div class="card">
<h3>필드 필터링</h3>
<p>password, secret 등 민감 필드를 자동 제거합니다. 중첩 객체도 지원합니다.</p>
</div>
<div class="card">
<h3>민감 정보 마스킹</h3>
<p>token, api_key 값을 부분 마스킹하여 로그에 안전하게 남깁니다.</p>
</div>
<div class="card">
<h3>구조 변환</h3>
<p>키 이름 변경, 값 매핑으로 클라이언트가 원하는 형태로 변환합니다.</p>
</div>
<div class="card">
<h3>DoS 방어</h3>
<p>페이로드 크기·중첩 깊이·배열 요소 수를 제한하여 악의적 요청을 차단합니다.</p>
</div>
<div class="card">
<h3>Glob 룰 매칭</h3>
<p><code>/api/*</code>, <code>/api/**</code> 패턴으로 경로별 룰을 유연하게 설정합니다.</p>
</div>
</div>
</section>

<section>
<h2>API</h2>

<div class="card">
<h3>POST /process</h3>
<p>JSON 본체를 전송하면 룰에 따라 검증·필터링·마스킹·변환 후 결과를 반환합니다.</p>
<pre><span class="c"># 요청</span>
POST /process
Content-Type: application/json
<span class="k">x-ironjson-path</span>: /api/users
<span class="k">x-ironjson-direction</span>: request

<span class="c">// 본체</span>
{<span class="s">"email"</span>:<span class="s">"test@test.com"</span>,<span class="s">"password"</span>:<span class="s">"secret123"</span>,<span class="s">"token"</span>:<span class="s">"sk-abc123"</span>}

<span class="c"># 응답</span>
{
  <span class="s">"success"</span>: <span class="n">true</span>,
  <span class="s">"data"</span>: {
    <span class="s">"email"</span>: <span class="s">"test@test.com"</span>,
    <span class="s">"token"</span>: <span class="s">"*****c123"</span>
  }
}</pre>
</div>

<div class="card">
<h3>GET /health</h3>
<pre>{<span class="s">"status"</span>:<span class="s">"healthy"</span>,<span class="s">"service"</span>:<span class="s">"ironjson"</span>}</pre>
</div>

<div class="card">
<h3>에러 응답 형식</h3>
<pre>{
  <span class="s">"success"</span>: <span class="n">false</span>,
  <span class="s">"error"</span>: {
    <span class="s">"type"</span>: <span class="s">"validation"</span>,
    <span class="s">"message"</span>: <span class="s">"Validation failed"</span>,
    <span class="s">"details"</span>: [
      {<span class="s">"path"</span>:<span class="s">"$.email"</span>,<span class="s">"message"</span>:<span class="s">"Required field 'email' is missing"</span>}
    ]
  }
}</pre>
</div>
</section>

<section>
<h2>직접 테스트</h2>
<div class="try">
<textarea id="input" spellcheck="false">{
  "email": "user@example.com",
  "password": "my-secret-password",
  "token": "sk-live-abc123def456",
  "name": "John Doe",
  "age": 30
}</textarea>
<div class="row">
<select id="path">
<option value="/api/users">POST /api/users (검증 + 제거 + 마스킹)</option>
<option value="/api/*">/api/* (마스킹 전역 룰)</option>
</select>
<select id="dir">
<option value="request">Request</option>
<option value="response">Response</option>
</select>
<button id="btn" onclick="send()">실행</button>
</div>
<pre id="result" style="min-height:60px"><span class="c">// 결과가 여기에 표시됩니다</span></pre>
</div>
</section>

<section>
<h2>룰 설정 예시</h2>
<pre><span class="c">// IRONJSON_RULES 환경 변수로 주입</span>
{
  <span class="s">"rules"</span>: [
    {
      <span class="s">"path"</span>: <span class="s">"/api/users"</span>,
      <span class="s">"methods"</span>: [<span class="s">"POST"</span>, <span class="s">"PUT"</span>],
      <span class="s">"direction"</span>: <span class="s">"request"</span>,
      <span class="s">"validate"</span>: {
        <span class="s">"type"</span>: <span class="s">"object"</span>,
        <span class="s">"required"</span>: [<span class="s">"email"</span>],
        <span class="s">"properties"</span>: {
          <span class="s">"email"</span>: {<span class="s">"type"</span>:<span class="s">"string"</span>,<span class="s">"min_length"</span>:<span class="n">3</span>},
          <span class="s">"age"</span>:   {<span class="s">"type"</span>:<span class="s">"integer"</span>,<span class="s">"min"</span>:<span class="n">0</span>,<span class="s">"max"</span>:<span class="n">200</span>}
        }
      },
      <span class="s">"remove_fields"</span>: [<span class="s">"password"</span>, <span class="s">"secret"</span>],
      <span class="s">"mask_fields"</span>: [<span class="s">"token"</span>, <span class="s">"api_key"</span>]
    }
  ]
}</pre>
</section>

<footer>
IronJSON &mdash; Rust &bull; WebAssembly &bull; Cloudflare Workers<br>
MIT License
</footer>

</div>

<script>
async function send(){
  const btn=document.getElementById('btn');
  const pre=document.getElementById('result');
  const input=document.getElementById('input').value;
  const path=document.getElementById('path').value;
  const dir=document.getElementById('dir').value;
  btn.disabled=true;
  btn.textContent='처리 중...';
  pre.textContent='';
  try{
    const r=await fetch('/process',{
      method:'POST',
      headers:{
        'Content-Type':'application/json',
        'x-ironjson-path':path,
        'x-ironjson-direction':dir
      },
      body:input
    });
    const j=await r.json();
    pre.textContent=JSON.stringify(j,null,2);
  }catch(e){
    pre.textContent='Error: '+e.message;
  }finally{
    btn.disabled=false;
    btn.textContent='실행';
  }
}
</script>
</body>
</html>"##;

# IronJSON — Edge JSON Accelerator

Cloudflare Workers에서 동작하는 Rust 기반 초고속 JSON 처리 엔진.

API 요청/응답의 JSON에 대해 **스키마 검증 · 필드 제거 · 민감정보 마스킹 · 키 이름 변경 · 값 매핑**을 엣지에서 수행합니다.

## 아키텍처

```
Client Request (JSON)
  → Worker Entry → Rule Matcher → Safe JSON Parser
  → [Validate] → [Remove Fields] → [Mask Fields] → [Transform]
  → Serialized Response
```

### 모듈 구조

```
src/
├── lib.rs            #[event(fetch)] 진입점
├── handler.rs        HTTP 라우팅 & 요청 처리
├── config.rs         상수 (페이로드 제한, 마스킹 설정)
├── error.rs          통일 에러 타입 + 구조화된 JSON 에러 응답
├── engine/
│   ├── mod.rs        파이프라인 오케스트레이션
│   ├── parser.rs     안전 JSON 파싱 (사이즈·깊이·구조 제한)
│   ├── filter.rs     필드 제거 / 포함
│   ├── mask.rs       민감 정보 마스킹
│   ├── transform.rs  key rename, value mapping
│   └── validator.rs  경량 JSON Schema 검증
└── rule/
    ├── mod.rs        RuleEngine (룰 매칭 오케스트레이터)
    ├── config.rs     Rule/SchemaDef 타입 + 직렬화
    └── matcher.rs    Glob 패턴 매칭 (경량 자체 구현)
```

## 기술 선택

| 항목 | 선택 | 이유 |
|------|------|------|
| Worker SDK | `worker` (worker-rs) v0.4 | 공식 Rust SDK, wasm32 완벽 호환 |
| JSON | `serde` + `serde_json` | 표준, wasm32 검증, zero-copy 지원 |
| Glob 매칭 | 자체 구현 | `regex` crate는 WASM에서 무거움 |
| Schema 검증 | 자체 구현 | `jsonschema`는 의존성 과다, WASM 부적합 |

## 기능

- **JSON Schema 검증** — type, required, min/max, min_length, max_length, pattern 지원
- **필드 필터링** — remove_fields (제거), include_fields (포함)
- **민감 정보 마스킹** — password, token, api_key 등 자동 마스킹 (부분 노출 또는 전체 마스킹)
- **구조 변환** — key rename, value mapping
- **DoS 방어** — 페이로드 크기 제한, 중첩 깊이 제한, 배열/객체 요소 수 제한, 문자열 길이 제한
- **Malformed JSON 방어** — UTF-8 검증, 구조 사전 검사

## 설정

룰은 JSON으로 정의하며, Cloudflare Workers 환경 변수 `IRONJSON_RULES`로 주입하거나 기본 룰(`src/config.rs`)을 사용합니다.

```json
{
  "rules": [
    {
      "path": "/api/users",
      "methods": ["POST", "PUT", "PATCH"],
      "direction": "request",
      "validate": {
        "type": "object",
        "required": ["email"],
        "properties": {
          "email": { "type": "string", "min_length": 3 },
          "name":  { "type": "string" },
          "age":   { "type": "integer", "min": 0, "max": 200 }
        }
      },
      "remove_fields": ["password", "password_confirm", "secret"],
      "mask_fields": ["token", "api_key", "credit_card"],
      "rename": {},
      "value_map": {}
    },
    {
      "path": "/api/users/*",
      "methods": ["GET"],
      "direction": "response",
      "remove_fields": ["password", "internal_id"],
      "mask_fields": ["email", "phone"],
      "rename": { "internal_id": "id" },
      "value_map": {}
    },
    {
      "path": "/api/*",
      "methods": ["GET", "POST", "PUT", "PATCH", "DELETE"],
      "direction": "both",
      "mask_fields": ["token", "secret", "api_key"],
      "remove_fields": [],
      "rename": {},
      "value_map": {}
    }
  ]
}
```

### Glob 패턴

| 패턴 | 의미 |
|------|------|
| `/api/users` | 정확히 일치 |
| `/api/*` | 단일 세그먼트 와일드카드 |
| `/api/**` | 다중 세그먼트 와일드카드 |
| `/api/user-*` | 세그먼트 내 와일드카드 |

## API

### `POST /process`

요청 본체의 JSON에 룰을 적용합니다.

**요청 헤더:**

| 헤더 | 설명 | 기본값 |
|------|------|--------|
| `x-ironjson-path` | 룰 매칭에 사용할 API 경로 | `/api/*` |
| `x-ironjson-direction` | `request` · `response` · `both` | `request` |
| `Content-Type` | `application/json` (필수) | — |

**성공 응답:**

```json
{
  "success": true,
  "data": { ... }
}
```

**에러 응답:**

```json
{
  "success": false,
  "error": {
    "type": "validation",
    "message": "Validation failed",
    "details": [
      { "path": "$.email", "message": "Required field 'email' is missing", "expected": "present", "found": "missing" }
    ]
  }
}
```

### `GET /health`

헬스 체크 엔드포인트.

### `GET /`

서비스 정보 및 엔드포인트 목록.

## 빌드 및 배포

### 사전 요구

```bash
# Rust toolchain
rustup target add wasm32-unknown-unknown

# Wrangler CLI
npm install -g wrangler
```

### 빌드

```bash
cargo build --target wasm32-unknown-unknown --release
```

### 로컬 개발

```bash
wrangler dev
```

### 배포

```bash
wrangler deploy
```

### 환경 변수 설정 (선택)

```bash
wrangler secret put IRONJSON_RULES
```

## 성능 최적화 전략

| 전략 | 설명 |
|------|------|
| 최소 allocation | `serde_json::Value` 1회 생성 후 in-place 변환 |
| 불필요한 clone 금지 | `&mut Value` 기반 순회, clone은 최종 직렬화 시에만 |
| O(n) 처리 | 모든 연산이 JSON 트리를 1회 순회 |
| 컴파일 타임 상수 | 제한값을 `const`로 정의하여 런타임 오버헤드 제거 |
| LTO + size 최적화 | `opt-level = "z"`, LTO, codegen-units = 1 |
| DoS 방어 | 파싱 전 사이즈 검사, 깊이/요소수 제한으로 조기 거부 |

## 코딩 원칙

- `unsafe` 미사용
- `unwrap` 금지, 모든 경로 `Result` 기반 처리
- `clone`은 불가피한 경우에만 사용
- 명확한 타입 정의, Rust idiomatic 스타일

## 한계 및 개선 방향

| 항목 | 현재 | 개선 방향 |
|------|------|-----------|
| Schema 검증 | 경량 자체 구현 | `schemars` 기반 코드 생성 도입 고려 |
| Glob 매칭 | 단순 `*`/`**` 지원 | 고급 패턴(`?`, `[a-z]`) 확장 |
| Proxy 모드 | 미구현 | fetch 기반 upstream 프록시 추가 가능 |
| Value mapping | String→Value만 지원 | Number/Bool 매핑 확장 가능 |
| SIMD 파싱 | wasm32 미지원 | WASM SIMD 표준 안정화 후 `simd-json` 재검토 |

## 라이선스

MIT

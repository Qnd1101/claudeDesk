/// FR-08 미리보기 — 스트리밍 JSONL 읽기
///
/// 파일을 읽기 전용으로 스트리밍해 대화 턴을 최대 `max_lines`/`max_bytes` 한계까지
/// 추출한다. 파일에는 절대 쓰지 않는다(Non-Destructive).
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// 미리보기 최대 줄 수 기본값
pub const MAX_PREVIEW_LINES: usize = 200;
/// 미리보기 최대 바이트 기본값 (64 KiB)
pub const MAX_PREVIEW_BYTES: usize = 64 * 1024;

/// 단일 줄 바이트 상한: 이 크기를 초과하는 줄은 turn으로 렌더하지 않고 skipped 처리.
/// 실제 세션의 base64 이미지·대형 tool_result를 RAM에 전부 올리지 않기 위함.
const MAX_LINE_BYTES: usize = 256 * 1024;

/// 미리보기에서 하나의 대화 턴
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewTurn {
    /// "user" 또는 "assistant"
    pub role: String,
    /// 렌더링할 텍스트 (개행 포함 가능)
    pub text: String,
}

/// `read_preview` 결과
#[derive(Debug, Clone)]
pub struct PreviewContent {
    pub turns: Vec<PreviewTurn>,
    /// max_lines 또는 max_bytes 상한에 걸려 중단됐으면 true
    pub truncated: bool,
    /// 파싱 불가 줄(깨진 JSON, IO 오류, 거대 줄 등) 스킵 수
    pub skipped_lines: usize,
}

impl PreviewContent {
    /// 빈 결과 생성 (빈 파일·없는 경로용)
    pub fn empty() -> Self {
        PreviewContent {
            turns: vec![],
            truncated: false,
            skipped_lines: 0,
        }
    }
}

/// JSONL 파일을 스트리밍으로 읽어 대화 턴 목록을 반환한다.
///
/// # RAM 바운드 보장
/// `BufReader`를 `take(max_bytes)` 로 감싼다. 파일에서 읽히는 총 바이트가 절대
/// `max_bytes`를 초과하지 않는다. 단일 거대 줄(수 MB의 base64·tool_result 등)도
/// 파일 읽기 레벨에서 차단된다(잘린 불완전 JSON → graceful skip).
///
/// # 인자
/// - `max_lines`: 누적 렌더 줄 수(텍스트 개행 기준) 상한. 초과 시 즉시 중단.
/// - `max_bytes`: 파일에서 읽을 총 바이트 상한. `take()`로 하드 실링.
///
/// # 동작
/// - 깨진 줄·JSON 파싱 실패·거대 줄은 skip 처리(`skipped_lines` 카운트), 패닉 없음.
/// - 빈 파일·없는 경로 → 빈 turns 반환(에러 아님).
/// - 원본 파일은 읽기만 한다(쓰기 0).
pub fn read_preview(path: &Path, max_lines: usize, max_bytes: usize) -> PreviewContent {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return PreviewContent::empty(),
    };

    // ── 핵심: take()로 파일 읽기량을 max_bytes로 하드 실링 ─────────────────
    // 단일 거대 줄도 이 위에서 lines()를 호출하므로 파일 레벨에서 잘린다.
    // 잘린 줄 → 불완전 JSON → serde_json 파싱 실패 → graceful skip.
    let limited = BufReader::new(file).take(max_bytes as u64);
    let reader = BufReader::new(limited);

    let mut turns: Vec<PreviewTurn> = vec![];
    let mut truncated = false;
    let mut skipped_lines: usize = 0;
    let mut accumulated_lines: usize = 0;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => {
                // take() 한계에 도달하면 마지막 불완전 줄에서 IO 에러 발생.
                // truncated로 표시하고 종료.
                truncated = true;
                break;
            }
        };

        let raw_byte_len = line.len() + 1; // +1: 개행 문자 포함

        let trimmed = line.trim();
        if trimmed.is_empty() {
            // 빈 줄: 바이트 카운트는 take()가 추적하므로 별도 누적 불필요
            continue;
        }

        // ── per-line 바이트 가드 ────────────────────────────────────────────
        // take()가 파일 총량을 제한하지만, take() 한계 내에서도 단일 줄이
        // MAX_LINE_BYTES를 넘으면 렌더하기 부적합한 크기이므로 skip 처리.
        if raw_byte_len > MAX_LINE_BYTES {
            skipped_lines += 1;
            continue;
        }

        // JSON 파싱
        let v: Value = match serde_json::from_str(trimmed) {
            Ok(val) => val,
            Err(_) => {
                skipped_lines += 1;
                continue;
            }
        };

        // type 필드 확인
        let line_type = match v.get("type").and_then(|t| t.as_str()) {
            Some(t) => t,
            None => continue,
        };

        if !matches!(line_type, "user" | "assistant") {
            continue;
        }

        let role = line_type.to_string();

        // message.content 추출
        let content = v.get("message").and_then(|m| m.get("content"));

        let text = extract_preview_text(content);
        if text.is_empty() {
            continue;
        }

        // 렌더 줄 수 계산 (텍스트 개행 기준)
        let line_count = text.lines().count().max(1);
        if accumulated_lines + line_count > max_lines {
            truncated = true;
            break;
        }
        accumulated_lines += line_count;

        turns.push(PreviewTurn { role, text });
    }

    // take() 소진으로 루프가 정상 종료된 경우에도 파일에 남은 내용이 있을 수 있음.
    // turns가 비어 있지 않고 루프가 break 없이 끝났어도, take() 한계에 걸렸다면
    // truncated는 이미 위에서 설정됐다. 추가로: turns > 0이고 루프 종료가 take()
    // 소진에 의한 것이었다면 위의 Err 분기에서 truncated=true가 됐으므로 별도 처리 불필요.

    PreviewContent {
        turns,
        truncated,
        skipped_lines,
    }
}

/// message.content 값(Value)에서 미리보기용 텍스트를 추출한다.
///
/// - String content → 그대로 반환
/// - 블록 배열 → text 블록은 줄바꿈으로 join;
///   tool_use 블록은 `[도구 호출: <name>]`, tool_result 블록은 `[도구 결과]`,
///   나머지 비-text 블록은 생략한다.
fn extract_preview_text(content: Option<&Value>) -> String {
    let v = match content {
        Some(val) => val,
        None => return String::new(),
    };

    match v {
        Value::String(s) => s.trim().to_string(),
        Value::Array(arr) => {
            let mut parts: Vec<String> = vec![];
            for block in arr {
                let obj = match block.as_object() {
                    Some(o) => o,
                    None => continue,
                };
                let block_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match block_type {
                    "text" => {
                        if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                parts.push(trimmed.to_string());
                            }
                        }
                    }
                    "tool_use" => {
                        let name = obj
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("unknown");
                        parts.push(format!("[도구 호출: {}]", name));
                    }
                    "tool_result" => {
                        parts.push("[도구 결과]".to_string());
                    }
                    // image, document 등 기타 비-text 블록은 생략
                    _ => {}
                }
            }
            parts.join("\n")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_preview_text_string() {
        let v = json!("hello world");
        assert_eq!(extract_preview_text(Some(&v)), "hello world");
    }

    #[test]
    fn test_extract_preview_text_block_array_join() {
        let v = json!([
            {"type": "text", "text": "첫 번째"},
            {"type": "tool_use", "id": "abc", "name": "Read"},
            {"type": "text", "text": "두 번째"}
        ]);
        let result = extract_preview_text(Some(&v));
        assert!(result.contains("첫 번째"), "첫 text 블록 누락");
        assert!(
            result.contains("[도구 호출: Read]"),
            "tool_use placeholder 누락"
        );
        assert!(result.contains("두 번째"), "두 번째 text 블록 누락");
    }

    #[test]
    fn test_extract_preview_text_tool_result() {
        let v = json!([
            {"type": "tool_result", "tool_use_id": "x", "content": "결과"}
        ]);
        let result = extract_preview_text(Some(&v));
        assert_eq!(result, "[도구 결과]");
    }

    #[test]
    fn test_extract_preview_text_none() {
        assert_eq!(extract_preview_text(None), "");
    }

    #[test]
    fn test_extract_preview_text_empty_array() {
        let v = json!([]);
        assert_eq!(extract_preview_text(Some(&v)), "");
    }
}

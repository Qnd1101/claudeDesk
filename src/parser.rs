use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::time::SystemTime;

use crate::data::FileMeta;
use crate::domain::Session;

/// 최대 스캔 줄 수 (첫 user 줄 탐색 상한)
const MAX_SCAN_LINES: usize = 64;

/// 제목 최대 길이 (char 단위)
const TITLE_MAX_CHARS: usize = 80;

/// JSONL 한 줄의 최소 파싱 구조
#[derive(Debug, Deserialize)]
struct RawLine {
    #[serde(rename = "type")]
    line_type: Option<String>,
    timestamp: Option<String>,
    cwd: Option<String>,
    message: Option<RawMessage>,
}

#[derive(Debug, Deserialize)]
struct RawMessage {
    content: Option<Value>,
}

/// 파싱 결과
pub struct ParseResult {
    pub title: String,
    pub cwd: String,
    pub created: SystemTime,
    pub msg_count: usize,
    pub skipped_lines: usize,
}

/// JSONL 파일을 BufReader로 스트리밍 파싱
pub fn parse_session(file_meta: &FileMeta) -> Result<ParseResult> {
    let file = std::fs::File::open(&file_meta.path)?;
    let reader = BufReader::new(file);

    let mut title: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut created: Option<SystemTime> = None;
    let mut msg_count: usize = 0;
    let mut skipped_lines: usize = 0;
    let mut scan_count: usize = 0;
    let mut found_user = false;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => {
                skipped_lines += 1;
                continue;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        scan_count += 1;

        let raw: RawLine = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                skipped_lines += 1;
                continue;
            }
        };

        // cwd: 첫 등장 시 저장
        if cwd.is_none() {
            if let Some(ref c) = raw.cwd {
                if !c.is_empty() {
                    cwd = Some(c.clone());
                }
            }
        }

        // created: 첫 timestamp 보유 줄
        if created.is_none() {
            if let Some(ref ts) = raw.timestamp {
                if let Ok(dt) = ts.parse::<chrono::DateTime<chrono::Utc>>() {
                    created = Some(SystemTime::from(dt));
                }
            }
        }

        let line_type = raw.line_type.as_deref().unwrap_or("");

        // 메시지 수 집계 (user | assistant)
        if matches!(line_type, "user" | "assistant") {
            msg_count += 1;
        }

        // 제목 도출: 첫 user 줄 (상한 K줄 내)
        if !found_user && line_type == "user" && scan_count <= MAX_SCAN_LINES {
            if let Some(ref msg) = raw.message {
                if let Some(text) = extract_text_from_content(msg.content.as_ref()) {
                    let trimmed_text = text.trim().to_string();
                    if !trimmed_text.is_empty() {
                        title = Some(truncate_title(&trimmed_text));
                        found_user = true;
                    }
                }
            }
        }

        // 상한 초과 시 제목 탐색 종료 (나머지 줄은 계속 카운트)
        // 단, 이미 user를 찾았거나 scan_count > MAX_SCAN_LINES이면 탐색 중단하되 카운트 계속
    }

    Ok(ParseResult {
        title: title.unwrap_or_else(|| "Untitled Session".to_string()),
        cwd: cwd.unwrap_or_default(),
        created: created.unwrap_or(file_meta.ctime),
        msg_count,
        skipped_lines,
    })
}

/// message.content에서 첫 텍스트 추출
/// - content가 String이면 그 값
/// - content가 블록 배열이면 첫 {"type":"text","text":"..."} 블록의 text
pub fn extract_text_from_content(content: Option<&Value>) -> Option<String> {
    let v = content?;
    match v {
        Value::String(s) => {
            if s.is_empty() {
                None
            } else {
                Some(s.clone())
            }
        }
        Value::Array(arr) => {
            for block in arr {
                if let Some(obj) = block.as_object() {
                    let block_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if block_type == "text" {
                        if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                return Some(text.to_string());
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// 제목을 최대 TITLE_MAX_CHARS 자로 자름 (첫 비공백 줄)
fn truncate_title(text: &str) -> String {
    // 첫 비공백 줄
    let first_line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or(text);
    let trimmed = first_line.trim();

    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= TITLE_MAX_CHARS {
        trimmed.to_string()
    } else {
        let truncated: String = chars[..TITLE_MAX_CHARS].iter().collect();
        format!("{}…", truncated.trim_end())
    }
}

/// FileMeta + ParseResult → Session 조립
pub fn build_session(
    file_meta: &FileMeta,
    result: ParseResult,
    active_window_secs: u64,
) -> Session {
    let session_id = file_meta
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // cwd: 레코드 cwd 우선, 없으면 폴더명 역치환
    let cwd = if result.cwd.is_empty() {
        // 부모 폴더명으로 역치환
        file_meta
            .path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(crate::config::folder_name_to_cwd)
            .unwrap_or_default()
    } else {
        result.cwd
    };

    // 활성 세션 판정: mtime이 now - active_window_secs 이내
    let is_active = is_recently_modified(&file_meta.mtime, active_window_secs);

    Session {
        session_id,
        title: result.title,
        cwd,
        created: result.created,
        modified: file_meta.mtime,
        msg_count: result.msg_count,
        is_active,
        path: file_meta.path.clone(),
        skipped_lines: result.skipped_lines,
    }
}

fn is_recently_modified(mtime: &SystemTime, window_secs: u64) -> bool {
    if let Ok(elapsed) = mtime.elapsed() {
        elapsed.as_secs() <= window_secs
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_text_string() {
        let v = json!("hello world");
        assert_eq!(
            extract_text_from_content(Some(&v)),
            Some("hello world".to_string())
        );
    }

    #[test]
    fn test_extract_text_block_array() {
        let v = json!([
            {"type": "tool_use", "id": "abc"},
            {"type": "text", "text": "첫 번째 텍스트 블록"}
        ]);
        assert_eq!(
            extract_text_from_content(Some(&v)),
            Some("첫 번째 텍스트 블록".to_string())
        );
    }

    #[test]
    fn test_extract_text_empty_array() {
        let v = json!([{"type": "tool_use", "id": "abc"}]);
        assert_eq!(extract_text_from_content(Some(&v)), None);
    }

    #[test]
    fn test_extract_text_none() {
        assert_eq!(extract_text_from_content(None), None);
    }

    #[test]
    fn test_truncate_title_short() {
        let t = truncate_title("짧은 제목");
        assert_eq!(t, "짧은 제목");
    }

    #[test]
    fn test_truncate_title_long() {
        let long = "a".repeat(100);
        let t = truncate_title(&long);
        let char_count: usize = t.chars().count();
        // 80자 + "…" 접미사
        assert!(char_count <= 82);
    }

    #[test]
    fn test_truncate_title_multiline() {
        let t = truncate_title("\n\n첫 번째 줄\n두 번째 줄");
        assert_eq!(t, "첫 번째 줄");
    }
}

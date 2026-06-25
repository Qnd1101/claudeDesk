use std::path::PathBuf;
use std::time::SystemTime;

/// 세션 도메인 모델
#[derive(Debug, Clone)]
pub struct Session {
    /// 파일명(확장자 제외) = sessionId (UUID)
    pub session_id: String,
    /// 도출된 제목 (첫 user 텍스트 80자 또는 Untitled Session)
    pub title: String,
    /// 작업 디렉토리 (레코드 cwd 필드 우선, 없으면 폴더명 역치환)
    pub cwd: String,
    /// 생성시각 (첫 timestamp, 없으면 파일 ctime). FR-07 정렬(created key)에 사용.
    pub created: SystemTime,
    /// 최종수정시각 (파일 mtime)
    pub modified: SystemTime,
    /// 메시지 수 (type: user|assistant 카운트)
    pub msg_count: usize,
    /// 활성 세션 여부 (mtime 근접 휴리스틱)
    pub is_active: bool,
    /// 파일 경로. M2 삭제/휴지통(FR-04)에서 사용 예정.
    #[allow(dead_code)]
    pub path: PathBuf,
    /// 파싱 중 스킵된 줄 수(세션별 진단). 현재는 집계 stats로 표시.
    #[allow(dead_code)]
    pub skipped_lines: usize,
    /// 검색 대상 텍스트: title + cwd 결합 (FR-05 incremental 필터용)
    pub search_text: String,
}

/// cwd 문자열에서 마지막 경로 세그먼트를 반환 (/ 또는 \\ 분리)
pub fn project_name_of(cwd: &str) -> &str {
    cwd.rsplit(['/', '\\']).next().unwrap_or(cwd)
}

impl Session {
    /// 표시용 프로젝트명: cwd 마지막 세그먼트
    pub fn project_name(&self) -> &str {
        project_name_of(&self.cwd)
    }
}

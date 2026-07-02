use std::{fmt::{self}, str::FromStr};

use serde::{Serialize};
use chrono::Utc;
use sqlx::{FromRow, MySqlPool, mysql::MySqlQueryResult};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub user_id: i64,
    pub user_name: String,
    pub auth_uid: String
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserRanking {
    pub user_id: i64,
    pub user_name: String,
    pub points: i32,
    pub penalty: i32
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct FinalRanking {
    pub user_id: i64,
    pub user_name: String,
    pub points: i32,
    pub penalty: i32,
    pub problems: Vec<PublicRankingItems>
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct RankingItems{
    pub user_id: i64,
    pub user_name: String,
    pub points: i32,
    pub penalty: i32,
    pub problem_id: i64,
    pub problem_order: String,
    pub accepted: bool
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PublicRankingItems{
    pub problem_id: i64,
    pub problem_order: String,
    pub accepted: bool
}

impl PublicRankingItems{
    fn from_private(item: RankingItems) -> Self{
        Self { problem_id: item.problem_id, 
            problem_order: item.problem_order, 
            accepted: item.accepted 
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Contest{
    pub contest_id: i64,
    pub contest_name: String,
    pub host: String,
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: chrono::DateTime<Utc>,
    pub is_active: bool,
    pub registered: bool,
    pub registered_count: i64
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Problem {
    pub problem_id: i64,
    pub problem_name: String,
    pub runtime_ms: i64,
    pub memory_mb: i64,
    pub problem_rating: i32,
    pub problem_statement: String,
    pub judge_type: String,
    pub validator_code: Option<String>
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PublicProblem{
    pub problem_id: i64,
    pub problem_name: String,
    pub runtime_ms: i64,
    pub memory_mb: i64,
    pub problem_rating: i32,
    pub problem_statement: String,
    pub judge_type: String,
    pub accepted: bool,
    pub problem_order: Option<String>
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TestCase {
    pub problem_id: i64,
    pub input: String,
    pub solution: String
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Submission {
    pub submission_id: i64,
    pub user_id: i64,
    pub contest_id: Option<i64>,
    pub problem_id: i64,
    pub verdict: String,
    pub runtime_ms: Option<i64>,
    pub memory_kb: Option<i64>,
    pub language: Option<String>,
    pub source_code: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct SubmissionStatus{
    pub submission_id: i64,
    pub user_id: i64,
    pub user_name: String,
    pub problem_id: i64,
    pub verdict: String,
    pub runtime_ms: Option<i64>,
    pub memory_kb: Option<i64>,
    pub language: Option<String>,
    pub submitted_time: chrono::DateTime<Utc>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pending,
    Accepted,
    WrongAnswer,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    RunTimeError,
    CompileError,
    JudgeFailure,
    Judging
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVerdictError;

impl fmt::Display for ParseVerdictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected one of PENDING, JUDGING, AC, WA, TLE, MLE, RE, CE Or JF")
    }
}

impl std::error::Error for ParseVerdictError {}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Judging => "JUDGING",
            Self::Accepted => "AC",
            Self::WrongAnswer => "WA",
            Self::TimeLimitExceeded => "TLE",
            Self::MemoryLimitExceeded => "MLE",
            Self::RunTimeError => "RE",
            Self::CompileError => "CE",
            Self::JudgeFailure => "JF"
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Verdict {
    type Err = ParseVerdictError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PENDING" => Ok(Self::Pending),
            "JUDGING" => Ok(Self::Judging),
            "AC" => Ok(Self::Accepted),
            "WA" => Ok(Self::WrongAnswer),
            "TLE" => Ok(Self::TimeLimitExceeded),
            "MLE" => Ok(Self::MemoryLimitExceeded),
            "RE" => Ok(Self::RunTimeError),
            "CE" => Ok(Self::CompileError),
            "JF" => Ok(Self::JudgeFailure),
            _ => Err(ParseVerdictError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language{
    GCC14,
    PYTHON3_12,
}
#[derive(Debug)]
pub struct LanguageNotSupportedError;
impl fmt::Display for LanguageNotSupportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Selected langauge is not supported")
    }
}
impl std::error::Error for LanguageNotSupportedError{}

impl Language{
    pub fn as_img(&self) -> &'static str{
        match self{
            Self::GCC14 => "gcc:14",
            Self::PYTHON3_12 => "python:3.12-slim"
        }
    }

    pub fn as_exten(&self) -> &'static str {
        match self {
            Self::GCC14 => ".cpp",
            Self::PYTHON3_12 => ".py"
        }
    }

    pub fn compile_command(&self, file_name:&str, compiled_file:&str) -> Vec<String>{
        match self{
            Self::GCC14 => {
                vec![
                    String::from("g++"),
                    String::from("-O2"),
                    String::from("-fsanitize=address,undefined"),
                    String::from("-fno-sanitize-recover=all"),
                    file_name.to_string(),
                    String::from("-o"),
                    compiled_file.to_string()
                ]
            }
            Self::PYTHON3_12 => {
                vec![]
            }
        }
    }

    pub fn run_command(&self, source_code:&str, compiled_file:&str, test_cases: &str) -> String{
        match self{
            Self::GCC14 => format!(
                r#"./"{}" < "/app/inputs/{}"; EXIT_CODE=$?; M=$(cat /sys/fs/cgroup/memory.current 2>/dev/null || cat /sys/fs/cgroup/memory/memory.usage_in_bytes 2>/dev/null); echo "JUDGE_MEM:$M" >&2; exit $EXIT_CODE"#, 
            compiled_file, test_cases),
            Self::PYTHON3_12 => format!(
                r#"python3 "{}" < "/app/inputs/{}"; EXIT_CODE=$?; M=$(cat /sys/fs/cgroup/memory.current 2>/dev/null); echo "JUDGE_MEM:$M" >&2; exit $EXIT_CODE"#,
                source_code, test_cases)
        }
    }
}

impl FromStr for Language{
    type Err = LanguageNotSupportedError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s{
            "c++20" => Ok(Self::GCC14),
            "python3" => Ok(Self::PYTHON3_12),
            _ => Err(LanguageNotSupportedError)
        }
    }
}

#[derive(Debug)]
pub enum SubmissionError{
    SubmissionLimitExceeded(String),
    TransactionFailed(sqlx::Error),
    SubmissionCoolDown(String),
}

impl From<sqlx::Error> for SubmissionError {
    fn from(error: sqlx::Error) -> Self {
        Self::TransactionFailed(error)
    }
}

pub async fn get_users(pool: &MySqlPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, user_name, auth_uid
        FROM users
        ORDER BY user_id
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn get_user(pool: &MySqlPool, user_id: i64) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, user_name, auth_uid
        FROM users
        WHERE user_id = ?
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_name(
    pool: &MySqlPool,
    user_name: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, user_name, auth_uid
        FROM users
        WHERE user_name = ?
        "#,
    )
    .bind(user_name)
    .fetch_optional(pool)
    .await
}

pub async fn get_user_by_uid(
    pool: &MySqlPool,
    auth_uid: &str,
) -> Result<Option<User>,sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, user_name, auth_uid
        FROM users
        WHERE auth_uid = ?
        "#,
    )
    .bind(auth_uid)
    .fetch_optional(pool)
    .await
}

pub async fn insert_user(pool: &MySqlPool, user_name: &str, auth_uid:&str) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO users (user_name, auth_uid)
        VALUES (?, ?)
        "#,
    )
    .bind(user_name)
    .bind(auth_uid)
    .execute(pool)
    .await?;

    Ok(last_insert_id(result))
}
pub async fn create_contest(
    pool: &MySqlPool,
    contest_name: &str,
    host: &str,
    start_time: &chrono::DateTime<Utc>,
    end_time: &chrono::DateTime<Utc>,
) -> Result<i64, sqlx::Error>{
    let result = sqlx::query(r#"
        INSERT INTO contests (contest_name, host, start_time, end_time)
        VALUES(?, ?, ?, ?)
    "#)
    .bind(contest_name)
    .bind(host)
    .bind(start_time)
    .bind(end_time)
    .execute(pool)
    .await?;

    Ok(last_insert_id(result))
}

pub async fn assign_contest_problems(
    pool: &MySqlPool,
    contest_id: i64,
    problem_id: i64,
    problem_order: &str,
) -> Result<u64, sqlx::Error>{
    let result = sqlx::query(r#"
        INSERT IGNORE INTO contest_problems (contest_id, problem_id, problem_order)
        VALUES(? , ? , ?)
    "#)
    .bind(contest_id)
    .bind(problem_id)
    .bind(problem_order)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_contest_problems(
    pool: &MySqlPool,
    user_id: Option<i64>,
    contest_id: i64
) -> Result<Vec<PublicProblem>, sqlx::Error>{
    let user_id = user_id.unwrap_or(0);

    sqlx::query_as::<_,PublicProblem>(r#"
        SELECT p.problem_id, p.problem_name, p.runtime_ms,
        p.memory_mb, p.problem_rating, p.problem_statement,
        p.judge_type, 
        EXISTS(
            SELECT 1 FROM submissions s
            WHERE s.problem_id = p.problem_id
            AND s.contest_id = cp.contest_id
            AND s.user_id = ?
            AND s.verdict = 'AC'
        ) as accepted,
        cp.problem_order
        FROM problems p JOIN contest_problems cp
        ON p.problem_id = cp.problem_id
        WHERE cp.contest_id = ?
        ORDER BY cp.problem_order ASC
    "#)
    .bind(user_id)
    .bind(contest_id)
    .fetch_all(pool)
    .await
}

pub async fn register_contest(
    pool: &MySqlPool,
    contest_id: i64,
    user_id: i64,
) -> Result<u64, sqlx::Error>{
    let result = sqlx::query(r#"
        INSERT IGNORE INTO contest_registrations (contest_id, user_id)
        VALUES (?, ?)
    "#
    )
    .bind(contest_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_contest(
    pool: &MySqlPool,
    contest_id: i64,
    user_id: Option<i64>
) -> Result<Option<Contest>, sqlx::Error>{
    let user_id = user_id.unwrap_or(0);

    sqlx::query_as::<_,Contest>(r#"
        SELECT c.contest_id, c.contest_name, c.host, c.start_time, c.end_time, c.is_active, 
        EXISTS(
            SELECT 1 FROM contest_registrations cr
            WHERE cr.user_id = ? 
            AND cr.contest_id = c.contest_id
        ) as registered,
        (SELECT COUNT(*) FROM contest_registrations cr1
        WHERE cr1.contest_id = c.contest_id) as registered_count
        FROM contests c
        WHERE c.contest_id = ?
    "#)
    .bind(user_id)
    .bind(contest_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_recent_contests(
    pool: & MySqlPool,
    user_id: Option<i64>,
    limit: i32
) -> Result<Vec<Contest>, sqlx::Error>{
    let user_id = user_id.unwrap_or(0);

    sqlx::query_as::<_,Contest>(r#"
            SELECT c.contest_id, c.contest_name, c.host, c.start_time, c.end_time, c.is_active,
                EXISTS(
                SELECT 1 FROM contest_registrations cr
                WHERE cr.user_id = ?
                AND cr.contest_id = c.contest_id
            ) as registered,
            (SELECT COUNT(*) FROM contest_registrations cr1
            WHERE cr1.contest_id = c.contest_id) as registered_count
            FROM contests c
            ORDER BY 
            CASE 
                WHEN is_active = TRUE THEN 0
                ELSE 1
            END ASC,
            start_time DESC
            LIMIT ?
    "#)
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn manage_contests(
    pool: MySqlPool
) -> Result<(),sqlx::Error>{
    let mut tx = pool.begin().await?;
    sqlx::query(r#"
        UPDATE contests SET is_active = FALSE
        WHERE is_active = TRUE 
        AND end_time <= NOW()
    "#)
    .execute(&mut *tx)
    .await?;

    let contest_id = sqlx::query_scalar::<_,i64>(r#"
        SELECT contest_id FROM contests
        WHERE is_active = FALSE
        AND start_time <= NOW()
        AND end_time > NOW()
        ORDER BY start_time ASC
        LIMIT 1;
    "#)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(contest) = contest_id{
        sqlx::query(r#"
            UPDATE contests SET is_active = TRUE
            WHERE contest_id = ?
        "#)
        .bind(contest)
        .execute(&mut *tx)
        .await?;
        
        sqlx::query(r#"
            UPDATE problems p
            JOIN contest_problems c
            ON p.problem_id = c.problem_id
            SET p.is_public = TRUE
            WHERE c.contest_id = ?
        "#)
        .bind(contest)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn find_registered_contest(
    pool: &MySqlPool,
    user_id: i64,
    contest_id: i64
) -> Result<Option<Contest>, sqlx::Error>{
    sqlx::query_as::<_,Contest>(r#"
        SELECT c.contest_id, c.contest_name, 
        c.host, c.start_time, c.end_time, c.is_active, TRUE as registered,
        (SELECT COUNT(*) FROM contest_registrations cr1 WHERE cr1.contest_id = c.contest_id) as registered_count
        FROM contests c JOIN contest_registrations cr
        ON c.contest_id = cr.contest_id
        WHERE cr.user_id = ?
        AND c.contest_id = ?
        LIMIT 1
    "#)
    .bind(user_id)
    .bind(contest_id)
    .fetch_optional(pool)
    .await
}

pub async fn problem_in_contest(
    pool: &MySqlPool,
    contest_id: i64,
    problem_id: i64
) -> Result<bool,sqlx::Error>{
    sqlx::query_scalar(r#"
        SELECT EXISTS(
            SELECT 1 
            FROM contest_problems
            WHERE contest_id = ?
            AND problem_id = ?
        )
    "#)
    .bind(contest_id)
    .bind(problem_id)
    .fetch_one(pool)
    .await
}

pub async fn insert_problem(
    pool: &MySqlPool,
    problem_name: &str,
    runtime_ms: i64,
    memory_mb: i64,
    problem_rating: i32,
    problem_statement: &str,
    judge_type: &str,
    validator_code: &str,
    is_public: bool
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO problems (problem_name, runtime_ms, memory_mb, problem_rating, problem_statement, judge_type, validator_code, is_public)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(problem_name)
    .bind(runtime_ms)
    .bind(memory_mb)
    .bind(problem_rating)
    .bind(problem_statement)
    .bind(judge_type)
    .bind(validator_code)
    .bind(is_public)
    .execute(pool)
    .await?;

    Ok(last_insert_id(result))
}

pub async fn get_problem(
    pool: &MySqlPool,
    problem_id: i64,
) -> Result<Option<Problem>, sqlx::Error> {
    sqlx::query_as::<_, Problem>(
        r#"
        SELECT problem_id, problem_name, runtime_ms, memory_mb, problem_rating, problem_statement, judge_type, validator_code
        FROM problems
        WHERE problem_id = ?
        "#,
    )
    .bind(problem_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_public_problem(
    pool: &MySqlPool,
    problem_id: i64,
    user_id: Option<i64>
) -> Result<Option<PublicProblem>, sqlx::Error> {
    let user_id = user_id.unwrap_or(0);

    sqlx::query_as::<_, PublicProblem>(
        r#"
        SELECT problem_id, problem_name, runtime_ms, memory_mb, problem_rating, problem_statement, judge_type,
        EXISTS(
            SELECT 1 FROM 
            submissions s
            WHERE p.problem_id = s.problem_id
            AND s.user_id = ?
            AND s.verdict ='AC'
            LIMIT 1
        ) AS accepted,
        NULL AS problem_order 
        FROM problems p
        WHERE p.problem_id = ? AND p.is_public = TRUE
        "#,
    )
    .bind(user_id)
    .bind(problem_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_contest_public_problem(
    pool: &MySqlPool,
    problem_id: i64,
    user_id: i64,
    contest_id: i64
) -> Result<Option<PublicProblem>, sqlx::Error>{

    sqlx::query_as::<_,PublicProblem>(r#"
        SELECT 
        p.problem_id,
        p.problem_name,
        p.runtime_ms,
        p.memory_mb,
        p.problem_rating,
        p.problem_statement,
        p.judge_type,
        EXISTS(
            SELECT 1 FROM submissions s WHERE
            s.problem_id = p.problem_id
            AND s.user_id = ?
            AND s.contest_id = cp.contest_id
            AND s.verdict = 'AC'
        ) as accepted,
        cp.problem_order
        FROM problems p JOIN contest_problems cp
        ON p.problem_id = cp.problem_id
        WHERE p.problem_id = ? AND
        cp.contest_id = ? AND
        p.is_public = TRUE
    "#)
    .bind(user_id)
    .bind(problem_id)
    .bind(contest_id)
    .fetch_optional(pool)  
    .await
}

pub async fn get_recent_problems(
    pool: &MySqlPool,
    user_id: Option<i64>,
    limit: i64,
) -> Result<Vec<PublicProblem>, sqlx::Error> {
    let user_id = user_id.unwrap_or(0);

    sqlx::query_as::<_, PublicProblem>(
        r#"
        SELECT 
            p.problem_id, 
            p.problem_name, 
            p.runtime_ms, 
            p.memory_mb,
            p.problem_rating,
            p.problem_statement,
            p.judge_type,
            Exists (
                SELECT 1
                FROM submissions s
                WHERE s.problem_id = p.problem_id
                    AND s.user_id = ?
                    AND s.verdict = 'AC'
                LIMIT 1
            ) AS accepted,
            NULL AS problem_order
        FROM problems p
        WHERE p.is_public = TRUE
        ORDER BY p.problem_id DESC
        LIMIT ?;
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn insert_testcase(
    pool: &MySqlPool,
    problem_id: i64,
    testcases: &str,
    solution: &str
) -> Result<i64,sqlx::Error> {
        let result = sqlx::query(
        r#"
        INSERT INTO testcases (problem_id, input, solution)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(problem_id)
    .bind(testcases)
    .bind(solution)
    .execute(pool)
    .await?;
    Ok(last_insert_id(result))
}

pub async fn get_test_cases(
    pool: &MySqlPool,
    problem_id: i64
) -> Result<Vec<TestCase>,sqlx::Error>{
    sqlx::query_as::<_, TestCase>(
        r#"
        SELECT problem_id, input, solution
        FROM testcases 
        WHERE problem_id = ?
        "#,
    )
    .bind(problem_id)
    .fetch_all(pool)
    .await
}

pub async fn get_example_test(
    pool: &MySqlPool,
    problem_id: i64
) -> Result<Option<TestCase>, sqlx::Error>{
        sqlx::query_as::<_, TestCase>(
        r#"
        SELECT problem_id, input, solution
        FROM testcases 
        WHERE problem_id = ? LIMIT 1
        "#,
    )
    .bind(problem_id)
    .fetch_optional(pool)
    .await
}

pub async fn insert_submission(
    pool: &MySqlPool,
    user_id: i64,
    problem_id: i64,
    verdict: Verdict,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
    language: Option<&str>,
    source_code: &str,
    contest_id: Option<i64>
) -> Result<i64, SubmissionError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        SELECT user_id FROM users
        WHERE user_id = ?
        FOR UPDATE
        "#
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    let pending_count:i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM submissions
        WHERE user_id = ? AND (verdict = 'PENDING' OR verdict = 'JUDGING')
        "#
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if pending_count > 0 {
        return Err(SubmissionError::SubmissionLimitExceeded(
            format!("User {} already has ongoing submissions!", user_id)
        ));
    }

    let cooldown_count:i64 = sqlx::query_scalar(
        r#"
            SELECT COUNT(*) FROM submissions
            WHERE user_id = ?
            AND submitted_time >= NOW() - INTERVAL 10 SECOND
        "#
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if cooldown_count > 0{
        return Err(SubmissionError::SubmissionCoolDown(
            "Slow down! You are submitting too often!".to_string()
        ));
    }

    let result = sqlx::query(
        r#"
        INSERT INTO submissions
            (user_id, problem_id, verdict, runtime_ms, memory_kb, language, source_code, contest_id)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(problem_id)
    .bind(verdict.as_str())
    .bind(runtime_ms)
    .bind(memory_kb)
    .bind(language)
    .bind(source_code)
    .bind(contest_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?; 

    Ok(last_insert_id(result))
}

pub async fn get_submission(
    pool: &MySqlPool,
    submission_id: i64,
) -> Result<Option<Submission>, sqlx::Error> {
    sqlx::query_as::<_, Submission>(
        r#"
        SELECT
            submission_id,
            user_id,
            contest_id,
            problem_id,
            verdict,
            runtime_ms,
            memory_kb,
            language,
            source_code
        FROM submissions
        WHERE submission_id = ?
        "#,
    )
    .bind(submission_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_recent_submissions(
    pool: &MySqlPool,
    limit: i64,
) -> Result<Vec<SubmissionStatus>, sqlx::Error> {
    sqlx::query_as::<_, SubmissionStatus>(
        r#"
        SELECT
            submission_id,
            s.user_id,
            u.user_name,
            problem_id,
            verdict,
            runtime_ms,
            memory_kb,
            language,
            submitted_time
        FROM submissions s
        JOIN users u ON u.user_id = s.user_id
        ORDER BY s.submitted_time DESC, submission_id DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_recent_submissions_by_user(
    pool: &MySqlPool,
    user_id: i64,
    limit: i64,
) -> Result<Vec<SubmissionStatus>, sqlx::Error>{
sqlx::query_as::<_, SubmissionStatus>(
        r#"
        SELECT
            submission_id,
            s.user_id,
            u.user_name,
            problem_id,
            verdict,
            runtime_ms,
            memory_kb,
            language,
            submitted_time
        FROM submissions s
        JOIN users u ON u.user_id = s.user_id
        WHERE s.user_id = ?
        ORDER BY s.submitted_time DESC, submission_id DESC
        LIMIT ?
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_contest_submissions(
    pool: & MySqlPool,
    contest_id: i64,
    limit: i32
) -> Result<Vec<SubmissionStatus>, sqlx::Error>{
    sqlx::query_as::<_,SubmissionStatus>(r#"
        SELECT 
        s.submission_id, 
        s.user_id, 
        u.user_name,
        s.problem_id, 
        s.verdict, 
        s.runtime_ms, 
        s.memory_kb,
        s.language, 
        s.submitted_time
        FROM submissions s JOIN users u
        ON s.user_id = u.user_id
        WHERE s.contest_id = ?
        ORDER BY s.submitted_time DESC, s.submission_id DESC
        LIMIT ?
    "#)
    .bind(contest_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_contest_user_submissions(
    pool: &MySqlPool,
    contest_id: i64,
    user_id: i64, 
    limit: i32
) -> Result<Vec<SubmissionStatus>, sqlx::Error>{
    sqlx::query_as::<_,SubmissionStatus>(r#"
        SELECT 
        s.submission_id, 
        s.user_id, 
        u.user_name,
        s.problem_id, 
        s.verdict, 
        s.runtime_ms, 
        s.memory_kb,
        s.language,
        s.submitted_time
        FROM submissions s JOIN users u
        ON s.user_id = u.user_id
        WHERE s.contest_id = ?
        AND s.user_id = ?
        ORDER BY s.submitted_time DESC, s.submission_id DESC
        LIMIT ?
    "#)
    .bind(contest_id)
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn update_submission_verdict(
    pool: &MySqlPool,
    submission_id: i64,
    user_id: Option<i64>,
    contest_id: Option<i64>,
    problem_id: Option<i64>,
    verdict: Verdict,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    if let Some(user_id) = user_id 
    && let Some(contest_id) = contest_id
    && let Some(problem_id) = problem_id{
        sqlx::query(r#"
            SELECT user_id
            FROM users
            WHERE user_id = ?
            FOR UPDATE;
        "#)
        .bind(user_id)
        .execute(&mut * tx)
        .await?;

        let accepted: bool = sqlx::query_scalar(r#"
            SELECT  EXISTS(
                SELECT 1 FROM submissions
                WHERE user_id = ? AND
                problem_id = ? AND
                contest_id = ? AND
                submission_id <> ? AND
                verdict = 'AC'
            )
        "#)
        .bind(user_id)
        .bind(problem_id)
        .bind(contest_id)
        .bind(submission_id)
        .fetch_one(&mut * tx)
        .await?;


        if let Some(contest) = get_contest(pool, contest_id, None).await?{
            
            let mut points = 0;
            let mut penalty= 0;
            if !accepted{
                if verdict == Verdict::Accepted{
                    points = 1;
                    penalty = (Utc::now()-contest.start_time).as_seconds_f32() as i32;     
               } else if verdict != Verdict::JudgeFailure 
               && verdict != Verdict::Pending
               && verdict != Verdict::Judging{
                    penalty += 1;
               } 
               if points != 0 || penalty != 0{
                    sqlx::query(r#"
                    UPDATE contest_registrations 
                    SET points = points + ?,
                    penalty = penalty + ?
                    WHERE contest_id = ?
                    AND user_id = ?
                    "#)
                    .bind(points)
                    .bind(penalty)
                    .bind(contest_id)
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;
               }
            }
        }
    }

    let result = sqlx::query(
        r#"
        UPDATE submissions
        SET verdict = ?, runtime_ms = ?, memory_kb = ?
        WHERE submission_id = ?
        "#,
    )
    .bind(verdict.as_str())
    .bind(runtime_ms)
    .bind(memory_kb)
    .bind(submission_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(result.rows_affected())
}

pub async fn get_contest_rankings(
    pool: &MySqlPool,
    contest_id: i64,
    limit: i64,
) -> Result<Vec<UserRanking>,sqlx::Error>{
    sqlx::query_as::<_,UserRanking>(r#"
        SELECT u.user_id, u.user_name, cr.points, cr.penalty
        FROM users u
        JOIN contest_registrations cr 
        ON u.user_id = cr.user_id
        WHERE cr.contest_id = ?
        ORDER BY 
        cr.points DESC, cr.penalty ASC, u.user_id ASC
        LIMIT ?
    "#)
    .bind(contest_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn get_contest_final_ranking(
    pool: & MySqlPool,
    contest_id: i64,
    limit: i64
) -> Result<Vec<FinalRanking>,sqlx::Error>{
    let items = sqlx::query_as::<_,RankingItems>(r#"
        SELECT 
        u.user_id,
        u.user_name,
        cr.points,
        cr.penalty,
        cp.problem_id, 
        cp.problem_order,
        EXISTS(
            SELECT 1 FROM 
            submissions s
            WHERE s.problem_id = cp.problem_id
            AND u.user_id = s.user_id
            AND s.contest_id = cp.contest_id
            AND s.verdict = 'AC'
        ) as accepted
        FROM
        (
            SELECT * FROM contest_registrations crsub
            WHERE contest_id = ?
            ORDER BY crsub.points DESC, 
            crsub.penalty ASC, 
            crsub.user_id ASC
            LIMIT ?
        ) cr 
        JOIN users u
        ON u.user_id = cr.user_id
        JOIN contest_problems cp ON
        cr.contest_id = cp.contest_id
        ORDER BY 
        cr.points DESC, 
        cr.penalty ASC, 
        u.user_id ASC,
        cp.problem_order ASC
    "#)
    .bind(contest_id)
    .bind(limit)
    .fetch_all(pool).await?;

    let mut final_ranks = Vec::new();
    let mut ranking_item = None;

    for item in items{
        match &mut ranking_item{
            None => {
                ranking_item = Some(FinalRanking{
                    user_id: item.user_id,
                    user_name: item.user_name.clone(),
                    points: item.points,
                    penalty: item.penalty,
                    problems: vec![PublicRankingItems::from_private(item)]
                })
            }
            Some(unwrapped_ranking_item) => {
                if unwrapped_ranking_item.user_id == item.user_id{
                    unwrapped_ranking_item.problems.push(PublicRankingItems::from_private(item));
                } else{
                    final_ranks.push(unwrapped_ranking_item.to_owned());
                    ranking_item = Some(FinalRanking { 
                        user_id: item.user_id, 
                        user_name: item.user_name.clone(), 
                        points: item.points,
                        penalty: item.penalty,
                        problems: vec![PublicRankingItems::from_private(item)] 
                    })
                }
            }
        } 
    }

    if let Some(item) = ranking_item{
        final_ranks.push(item.to_owned());
    }

    Ok(final_ranks)

}

pub async fn claim_next_pending(
    pool: &MySqlPool,
) -> Result<Option<Submission>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let pending_sub = sqlx::query_as::<_, Submission>(
        r#"
        SELECT
            submission_id,
            user_id,
            s.contest_id,
            problem_id,
            verdict,
            runtime_ms,
            memory_kb,
            language,
            source_code
        FROM submissions s
        LEFT JOIN contests c ON c.contest_id = s.contest_id
        WHERE verdict = 'PENDING'
        ORDER BY 
            CASE 
                WHEN c.is_active = TRUE THEN 0
                ELSE 1
            END ASC,
        submitted_time ASC, submission_id ASC
        LIMIT 1
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(sub) = &pending_sub{
        sqlx::query(
            r#"
            UPDATE submissions
            SET verdict = 'JUDGING'
            WHERE submission_id = ?
            "#
        )
        .bind(sub.submission_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(pending_sub)
}

pub async fn cleanup_submissions(
    pool: &MySqlPool
) -> Result<u64, sqlx::Error>{
    let result = sqlx::query(
        r#"
        UPDATE submissions
        SET verdict = 'PENDING'
        WHERE verdict = 'JUDGING'
        "#
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

fn last_insert_id(result: MySqlQueryResult) -> i64 {
    result.last_insert_id() as i64
}

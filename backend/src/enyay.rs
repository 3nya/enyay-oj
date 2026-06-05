use std::{fmt::{self}, str::FromStr};

use serde::Serialize;
use sqlx::{FromRow, MySqlPool, mysql::MySqlQueryResult};

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub user_id: i64,
    pub user_name: String,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Problem {
    pub problem_id: i64,
    pub problem_name: String,
    pub runtime_ms: i64,
    pub memory_mb: i64,
    pub problem_rating: i32
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
    pub problem_id: i64,
    pub verdict: String,
    pub runtime_ms: Option<i64>,
    pub memory_kb: Option<i64>,
    pub language: Option<String>,
    pub source_code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pending,
    Accepted,
    WrongAnswer,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    RunTimeError,
    CompileError
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseVerdictError;

impl fmt::Display for ParseVerdictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected one of PENDING, AC, WA, TLE, or MLE")
    }
}

impl std::error::Error for ParseVerdictError {}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Accepted => "AC",
            Self::WrongAnswer => "WA",
            Self::TimeLimitExceeded => "TLE",
            Self::MemoryLimitExceeded => "MLE",
            Self::RunTimeError => "RE",
            Self::CompileError => "CE"
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
            "AC" => Ok(Self::Accepted),
            "WA" => Ok(Self::WrongAnswer),
            "TLE" => Ok(Self::TimeLimitExceeded),
            "MLE" => Ok(Self::MemoryLimitExceeded),
            "RE" => Ok(Self::RunTimeError),
            "CE" => Ok(Self::CompileError),
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

pub async fn get_users(pool: &MySqlPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT user_id, user_name
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
        SELECT user_id, user_name
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
        SELECT user_id, user_name
        FROM users
        WHERE user_name = ?
        "#,
    )
    .bind(user_name)
    .fetch_optional(pool)
    .await
}

pub async fn insert_user(pool: &MySqlPool, user_name: &str) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO users (user_name)
        VALUES (?)
        "#,
    )
    .bind(user_name)
    .execute(pool)
    .await?;

    Ok(last_insert_id(result))
}

pub async fn insert_problem(
    pool: &MySqlPool,
    problem_name: &str,
    runtime_ms: i64,
    memory_mb: i64,
    problem_rating: i32
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO problems (problem_name, runtime_ms, memory_mb, problem_rating)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(problem_name)
    .bind(runtime_ms)
    .bind(memory_mb)
    .bind(problem_rating)
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
        SELECT problem_id, problem_name, runtime_ms, memory_mb, problem_rating
        FROM problems
        WHERE problem_id = ?
        "#,
    )
    .bind(problem_id)
    .fetch_optional(pool)
    .await
}

pub async fn get_recent_problems(
    pool: &MySqlPool,
    limit: i64,
) -> Result<Vec<Problem>, sqlx::Error> {
    sqlx::query_as::<_, Problem>(
        r#"
        SELECT problem_id, problem_name, runtime_ms, memory_mb
        FROM problems
        ORDER BY problem_id ASC
        LIMIT ?
        "#,
    )
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



pub async fn insert_submission(
    pool: &MySqlPool,
    user_id: i64,
    problem_id: i64,
    verdict: Verdict,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
    language: Option<&str>,
    source_code: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO submissions
            (user_id, problem_id, verdict, runtime_ms, memory_kb, language, source_code)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(user_id)
    .bind(problem_id)
    .bind(verdict.as_str())
    .bind(runtime_ms)
    .bind(memory_kb)
    .bind(language)
    .bind(source_code)
    .execute(pool)
    .await?;

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
) -> Result<Vec<Submission>, sqlx::Error> {
    sqlx::query_as::<_, Submission>(
        r#"
        SELECT
            submission_id,
            user_id,
            problem_id,
            verdict,
            runtime_ms,
            memory_kb,
            language,
            source_code
        FROM submissions
        ORDER BY submitted_time DESC, submission_id DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn insert_submission_with_id(
    pool: &MySqlPool,
    submission_id: i64,
    user_id: i64,
    problem_id: i64,
    verdict: Verdict,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
    language: Option<&str>,
    source_code: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO submissions
            (
                submission_id,
                user_id,
                problem_id,
                verdict,
                runtime_ms,
                memory_kb,
                language,
                source_code
            )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(submission_id)
    .bind(user_id)
    .bind(problem_id)
    .bind(verdict.as_str())
    .bind(runtime_ms)
    .bind(memory_kb)
    .bind(language)
    .bind(source_code)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn update_submission_verdict(
    pool: &MySqlPool,
    submission_id: i64,
    verdict: Verdict,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
) -> Result<u64, sqlx::Error> {
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
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

fn last_insert_id(result: MySqlQueryResult) -> i64 {
    result.last_insert_id() as i64
}

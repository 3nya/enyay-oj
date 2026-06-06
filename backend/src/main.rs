mod enyay;
mod judge;

use std::{net::SocketAddr, str::FromStr};

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::net::TcpListener;

use crate::enyay::{Language, Verdict, insert_problem, insert_submission, insert_testcase};

#[derive(Clone)]
struct AppState {
    pool: MySqlPool,
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    NotFound(String),
    Database(sqlx::Error),
    Io(std::io::Error),
    Judge(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message),
            Self::Database(error) => {
                eprintln!("database error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database request failed".to_string(),
                )
            }
            Self::Io(error) => {
                eprintln!("server error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "server request failed".to_string(),
                )
            }
            Self::Judge(error) => {
                eprintln!("judge error: {error}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("judge failed: {error}"),
                )
            }
        };

        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct IdResponse {
    id: i64,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    user_name: String,
}

#[derive(Deserialize)]
struct CreateProblemRequest {
    problem_name: String,
    runtime_ms: i64,
    memory_mb: i64,
    problem_rating: i32
}

#[derive(Deserialize)]
struct CreateTestCaseRequest {
    problem_id: i64,
    testcases: String,
    solution: String
}

#[derive(Deserialize)]
struct CreateSubmissionRequest {
    submission_id: Option<i64>,
    user_id: i64,
    problem_id: i64,
    verdict: Option<String>,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
    language: Option<String>,
    source_code: String,
}

#[derive(Deserialize)]
struct UpdateVerdictRequest {
    verdict: String,
    runtime_ms: Option<i64>,
    memory_kb: Option<i64>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn get_users(State(state): State<AppState>) -> Result<Json<Vec<enyay::User>>, ApiError> {
    Ok(Json(enyay::get_users(&state.pool).await?))
}

async fn get_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> Result<Json<enyay::User>, ApiError> {
    let user = enyay::get_user(&state.pool, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("user {user_id} not found")))?;

    Ok(Json(user))
}

async fn get_user_by_name(
    State(state): State<AppState>,
    Path(user_name): Path<String>,
) -> Result<Json<enyay::User>, ApiError> {
    let user = enyay::get_user_by_name(&state.pool, &user_name)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("user {user_name} not found")))?;

    Ok(Json(user))
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    if payload.user_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "user_name cannot be empty".to_string(),
        ));
    }

    let id = enyay::insert_user(&state.pool, payload.user_name.trim()).await?;
    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

async fn create_problem(
    State(state): State<AppState>,
    Json(payload): Json<CreateProblemRequest>,
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    if payload.problem_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "problem_name cannot be empty".to_string(),
        ));
    }

    if payload.runtime_ms <= 0 || payload.memory_mb <= 0 {
        return Err(ApiError::BadRequest(
            "runtime_ms and memory_kb must be positive".to_string(),
        ));
    }

    let id = enyay::insert_problem(
        &state.pool,
        payload.problem_name.trim(),
        payload.runtime_ms,
        payload.memory_mb,
        payload.problem_rating
    )
    .await?;

    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

async fn create_testcase(
    State(state): State<AppState>,
    Json(payload): Json<CreateTestCaseRequest>
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    if payload.problem_id <= 0 {
        return Err(ApiError::BadRequest("Problem id must be positive".to_string()));
    }
    if payload.solution.trim().is_empty() || payload.testcases.trim().is_empty() {
        return Err(ApiError::BadRequest("Testcases and solutions must not be empty".to_string()));
    }
    let id = enyay::insert_testcase(
        &state.pool,
        payload.problem_id,
        &payload.testcases, 
        &payload.solution)
        .await?;
    Ok((StatusCode::CREATED,Json(IdResponse { id })))
}

async fn get_problem(
    State(state): State<AppState>,
    Path(problem_id): Path<i64>,
) -> Result<Json<enyay::Problem>, ApiError> {
    let problem = enyay::get_problem(&state.pool, problem_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("problem {problem_id} not found")))?;

    Ok(Json(problem))
}

async fn get_recent_problems(
    State(state): State<AppState>,
) -> Result<Json<Vec<enyay::Problem>>, ApiError> {
    Ok(Json(enyay::get_recent_problems(&state.pool, 20).await?))
}

async fn create_submission(
    State(state): State<AppState>,
    Json(payload): Json<CreateSubmissionRequest>,
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    if payload.source_code.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "source_code cannot be empty".to_string(),
        ));
    }

    let verdict = parse_verdict(payload.verdict.as_deref().unwrap_or("PENDING"))?;
    let language = payload.language.as_deref();

    let id = match payload.submission_id {
        Some(submission_id) => {
            enyay::insert_submission_with_id(
                &state.pool,
                submission_id,
                payload.user_id,
                payload.problem_id,
                verdict,
                payload.runtime_ms,
                payload.memory_kb,
                language,
                &payload.source_code,
            )
            .await?;

            submission_id
        }
        None => {
            enyay::insert_submission(
                &state.pool,
                payload.user_id,
                payload.problem_id,
                verdict,
                payload.runtime_ms,
                payload.memory_kb,
                language,
                &payload.source_code,
            )
            .await?
        }
    };

    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

async fn get_submission(
    State(state): State<AppState>,
    Path(submission_id): Path<i64>,
) -> Result<Json<enyay::Submission>, ApiError> {
    let submission = enyay::get_submission(&state.pool, submission_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("submission {submission_id} not found")))?;

    Ok(Json(submission))
}

async fn get_recent_submissions(
    State(state): State<AppState>,
) -> Result<Json<Vec<enyay::Submission>>, ApiError> {
    Ok(Json(enyay::get_recent_submissions(&state.pool, 20).await?))
}

async fn judge_submission(
    State(state): State<AppState>,
    Path(submission_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let submission = enyay::get_submission(&state.pool, submission_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("submission {submission_id} not found")))?;

    let judge_volume = judge::JudgeVolume::new()?;
    judge::judge_submission(&submission, &judge_volume, &state)
        .await
        .map_err(|error| ApiError::Judge(error.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn update_submission_verdict(
    State(state): State<AppState>,
    Path(submission_id): Path<i64>,
    Json(payload): Json<UpdateVerdictRequest>,
) -> Result<StatusCode, ApiError> {
    let verdict = parse_verdict(&payload.verdict)?;
    let rows_affected = enyay::update_submission_verdict(
        &state.pool,
        submission_id,
        verdict,
        payload.runtime_ms,
        payload.memory_kb,
    )
    .await?;

    if rows_affected == 0 {
        return Err(ApiError::NotFound(format!(
            "submission {submission_id} not found"
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

fn parse_verdict(value: &str) -> Result<Verdict, ApiError> {
    Verdict::from_str(value).map_err(|error| ApiError::BadRequest(error.to_string()))
}

// Main function

#[tokio::main]
async fn main() -> Result<(),ApiError>{
    load_env();
    let db_url = std::env::var("DB_URL").unwrap();

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    println!("connected to database");
    //insert_problem(&pool, "Triple T", 1000, 64, 4000).await?;
   /*let _ = insert_submission(&pool, 1, 1, Verdict::Pending, None, None, Some("c++20"), 
    r#"
    #include<iostream>
    int main(){
        int n; std::cin << n;
        return 0;
    }"#).await.expect("Failed to insert to db");*/

    let judge_volume = judge::JudgeVolume::new().unwrap();

    let app_state = AppState {pool};

    let test_subs: Json<Vec<enyay::Submission>> = get_recent_submissions(State(app_state.clone())).await.expect("Failed to retrieve submission");
    let most_recent = &test_subs.0[0];

    let result = judge::judge_submission(most_recent,&judge_volume,&app_state).await;
    match result {
        Ok(res) => {
            println!("{}",res.verdict);
            println!("{:?}",res.metrics);
        }
        Err(_) => println!("Could not complete request")
    }
    Ok(())
}
/*async fn main() -> Result<(), ApiError> {
    load_env();

    let db_url = std::env::var("DB_URL").unwrap();
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    println!("connecting to database at {db_url}");
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    println!("connected to database");

    let app = Router::new()
        .route("/health", get(health))
        .route("/users", get(get_users).post(create_user))
        .route("/users/by-name/{user_name}", get(get_user_by_name))
        .route("/users/{user_id}", get(get_user))
        .route("/problems", post(create_problem))
        .route("/problems/all", get(get_recent_problems))
        .route("/problems/{problem_id}", get(get_problem))
        .route("/submissions", post(create_submission))
        .route("/submissions/recent", get(get_recent_submissions))
        .route("/submissions/{submission_id}", get(get_submission))
        .route("/submissions/{submission_id}/judge", post(judge_submission))
        .route(
            "/submissions/{submission_id}/verdict",
            patch(update_submission_verdict),
        )
        .with_state(AppState { pool });

    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| ApiError::BadRequest(format!("invalid BIND_ADDR: {error}")))?;
    let listener = TcpListener::bind(addr).await?;

    println!("server listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}*/

fn load_env() {
    if dotenvy::dotenv().is_err() {
        dotenvy::from_filename("backend/.env").ok();
    }
}

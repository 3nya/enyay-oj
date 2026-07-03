mod enyay;
mod judge;

use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use axum::{
    Json, Router, extract::{Path, State}, http::{StatusCode, header}, response::{Html, IntoResponse, Response}, routing::{get, patch, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use tokio::{net::TcpListener, sync::Semaphore};

#[derive(Clone)]
struct AppState {
    pool: MySqlPool,
    judge_volume: judge::JudgeVolume,
    judge_limit:Arc<Semaphore>
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    NotFound(String),
    Database(sqlx::Error),
    Io(std::io::Error),
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

impl From<enyay::SubmissionError> for ApiError{
    fn from(error: enyay::SubmissionError) -> Self{
        match error {
            enyay::SubmissionError::SubmissionLimitExceeded(message) =>{
                return Self::BadRequest(message);
            }
            enyay::SubmissionError::SubmissionCoolDown(message) => {
                return Self::BadRequest(message);
            }
            enyay::SubmissionError::TransactionFailed(err) =>{
                return Self::Database(err)
            }
        }
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
    auth_uid: String,
}

#[derive(Deserialize)]
struct CreateContestRequest {
    contest_name: String,
    host: String,
    start_time: chrono::DateTime<Utc>,
    end_time: chrono::DateTime<Utc>
}

#[derive(Deserialize)]
struct CreateProblemRequest {
    problem_name: String,
    runtime_ms: i64,
    memory_mb: i64,
    problem_rating: i32,
    problem_statement: String,
    judge_type: String,
    validator_code: Option<String>,
    is_public: bool
}

#[derive(Deserialize)]
struct CreateTestCaseRequest {
    testcases: String,
    solution: String
}

#[derive(Deserialize)]
struct CreateSubmissionRequest {
    user_id: i64,
    problem_id: i64,
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

async fn frontend_index() -> Html<&'static str> {
    Html(include_str!("../../frontend/index.html"))
}

async fn frontend_styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../../frontend/styles.css"),
    )
}

async fn frontend_script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../../frontend/app.js"),
    )
}

async fn frontend_logo() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../../frontend/assets/enyayoj-logo.png").as_slice(),
    )
}

async fn frontend_mascot() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/png")],
        include_bytes!("../../frontend/assets/enyayoj-mascot.png").as_slice(),
    )
}

async fn frontend_favicon() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/x-icon")],
        include_bytes!("../../frontend/assets/favicon.ico").as_slice(),
    )
}

async fn get_example_test(
    State(state) : State<AppState>,
    Path(problem_id):Path<i64>
) -> Result<Json<enyay::TestCase>, ApiError> {
    let example = enyay::get_example_test(&state.pool, problem_id)
    .await?
    .ok_or_else(|| ApiError::NotFound("No Example Testcase found".to_string()))?;
    Ok(Json(example))
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

async fn get_user_by_uid(
    State(state): State<AppState>,
    Path(auth_uid): Path<String>,
) -> Result<Json<enyay::User>, ApiError> {
    let user = enyay::get_user_by_uid(&state.pool, &auth_uid)
    .await?
    .ok_or_else(|| ApiError::NotFound("uid not found".to_string()))?;
    Ok(Json(user))
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    let username = payload.user_name.trim();
    if username.is_empty() {
        return Err(ApiError::BadRequest(
            "user_name cannot be empty".to_string(),
        ));
    }
    validate_username(username)?;
    if payload.auth_uid.is_empty() {
        return Err(ApiError::BadRequest(
            "uid cannot be empty".to_string(),
        ))
    }
    let id = enyay::insert_user(&state.pool, username, &payload.auth_uid).await?;
    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

fn validate_username(username:&str) -> Result<(),ApiError>{
    if username.len() < 3 || username.len() > 20 {
        return Err(ApiError::BadRequest(
            "usernames must be between 3-20 characters".to_string(),
        ));
    }

    if !username.chars().next().is_some_and(|c| c.is_ascii_alphabetic()){
        return Err(ApiError::BadRequest(
            "usernames must start with a letter".to_string(),
        ));
    }
    if username.chars().next_back().is_some_and(|c| c == '_') {
        return Err(ApiError::BadRequest(
            "usernames cannot have trailing underscores".to_string(),
        ));
    }
    if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(ApiError::BadRequest(
            "usernames may only contain letters, numbers and underscores".to_string()
        ));
    }
    Ok(())
}

async fn create_problem(
    State(state): State<AppState>,
    Json(payload): Json<CreateProblemRequest>,
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    let judge_type = payload.judge_type.trim();

    if payload.problem_name.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "problem_name cannot be empty".to_string(),
        ));
    }

    if payload.problem_statement.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "problem statement cannot be empty".to_string()
        ));
    }

    if payload.runtime_ms <= 0 || payload.memory_mb <= 0 {
        return Err(ApiError::BadRequest(
            "runtime_ms and memory_kb must be positive".to_string(),
        ));
    }

    if judge_type != "standard" && judge_type != "validator" {
        return Err(ApiError::BadRequest(
            "One of the 2 modes must be selected, standard or validator".to_string(),
        ));
    }

    let validator_code = payload.validator_code.unwrap_or(String::from(""));
    if judge_type == "validator" && validator_code.trim().is_empty() {
        return Err(ApiError::BadRequest("validator_code is required for validator problems".to_string()));
    }
    
    let id = enyay::insert_problem(
        &state.pool,
        payload.problem_name.trim(),
        payload.runtime_ms,
        payload.memory_mb,
        payload.problem_rating,
        &payload.problem_statement,
        judge_type,
        &validator_code,
        payload.is_public
    )
    .await?;

    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

async fn create_testcase(
    State(state): State<AppState>,
    Path(problem_id): Path<i64>,
    Json(payload): Json<CreateTestCaseRequest>
) -> Result<(StatusCode, Json<IdResponse>), ApiError> {
    if problem_id <= 0 {
        return Err(ApiError::BadRequest("Problem id must be positive".to_string()));
    }
    if payload.solution.trim().is_empty() || payload.testcases.trim().is_empty() {
        return Err(ApiError::BadRequest("Testcases and solutions must not be empty".to_string()));
    }
    let id = enyay::insert_testcase(
        &state.pool,
        problem_id,
        &payload.testcases, 
        &payload.solution)
        .await?;
    Ok((StatusCode::CREATED,Json(IdResponse { id })))
}

async fn get_problem_for_user(
    State(state): State<AppState>,
    Path((problem_id,user_id)): Path<(i64,i64)>,
) -> Result<Json<enyay::PublicProblem>, ApiError> {
    let problem = enyay::get_public_problem(&state.pool, problem_id,Some(user_id))
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("problem {problem_id} not found")))?;
    Ok(Json(problem))
}

async fn get_problem(
    State(state): State<AppState>,
    Path(problem_id): Path<i64>,
) -> Result<Json<enyay::PublicProblem>, ApiError> {
    let problem = enyay::get_public_problem(&state.pool, problem_id,None)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("problem {problem_id} not found")))?;
    Ok(Json(problem))
}

async fn get_contest_problem(
    State(state): State<AppState>,
    Path((contest_id,problem_id, user_id)): Path<(i64,i64,i64)>
) -> Result<Json<enyay::PublicProblem>, ApiError>{
    if let Some(contest) = enyay::get_contest(&state.pool, contest_id, Some(user_id)).await?{
        if !contest.registered{
            return Err(ApiError::BadRequest(format!("you are not registered for contest {contest_id}")));
        } else{
            return Ok(Json(
                enyay::get_contest_public_problem(&state.pool, problem_id, user_id, contest_id)
                .await?
                .ok_or_else(|| ApiError::NotFound(format!("problem {problem_id} not found in contest {contest_id}")))?
            ));
        }
    }
    Err(ApiError::NotFound(format!("contest {contest_id} does not exist")))
}

async fn get_recent_problems(
    State(state): State<AppState>,
    user_id: Option<Path<i64>>
) -> Result<Json<Vec<enyay::PublicProblem>>, ApiError> {
    match user_id {
        Some(Path(user_id)) => return Ok(Json(enyay::get_recent_problems(&state.pool,Some(user_id), 20).await?)),
        None => return Ok(Json(enyay::get_recent_problems(&state.pool,None, 20).await?))
    }
}

async fn get_user_contest_problems(
    State(state): State<AppState>,
    Path((contest_id, user_id)): Path<(i64,i64)>
) -> Result<Json<Vec<enyay::PublicProblem>>, ApiError>{
    if enyay::get_contest(&state.pool, contest_id,Some(user_id)).await?.is_none(){
        return Err(ApiError::NotFound(format!("contest {} does not exist",contest_id)));
    }
    Ok(Json(enyay::get_contest_problems(&state.pool, Some(user_id),contest_id).await?))
}

async fn get_contest_problems(
    State(state): State<AppState>,
    Path(contest_id): Path<i64>
) -> Result<Json<Vec<enyay::PublicProblem>>, ApiError>{
    if enyay::get_contest(&state.pool, contest_id,None).await?.is_none(){
        return Err(ApiError::NotFound(format!("contest {} does not exist",contest_id)));
    }
    Ok(Json(enyay::get_contest_problems(&state.pool, None, contest_id).await?))
}

async fn create_contest(
    State(state): State<AppState>,
    Json(payload): Json<CreateContestRequest>
) -> Result<(StatusCode, Json<IdResponse>), ApiError>{
    if payload.contest_name.is_empty() {
        return Err(ApiError::BadRequest(
            "contests must have a name!".to_string()
        ))
    }

    if payload.start_time >= payload.end_time{
        return Err(ApiError::BadRequest(
            "contest start time must be before end time".to_string()
        ));
    }

    let now = Utc::now();
    if payload.start_time <= now{
        return Err(ApiError::BadRequest(
            "contests cannot be started before it has been created!".to_string()
        ));
    }
    if payload.end_time <= now{
        return Err(ApiError::BadRequest(
            "contests cannot be finished before it is created!".to_string()
        ));
    }

    let id = enyay::create_contest(
        &state.pool, 
        &payload.contest_name, 
        &payload.host, 
        &payload.start_time, 
        &payload.end_time
    ).await?;

    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

async fn get_recent_contests(
    State(state): State<AppState>
) -> Result<Json<Vec<enyay::Contest>>, ApiError>{
    Ok(Json(enyay::get_recent_contests(&state.pool, None,20).await?))
}

async fn get_recent_user_contests(
    State(state): State<AppState>,
    Path(user_id): Path<i64>
) -> Result<Json<Vec<enyay::Contest>>, ApiError>{
    Ok(Json(enyay::get_recent_contests(&state.pool, Some(user_id), 20).await?))
}

async fn assign_problem_to_contest(
    State(state): State<AppState>,
    Path((contest_id,problem_id,problem_order)): Path<(i64,i64,String)>
) -> Result<StatusCode,ApiError> {
    if problem_order.len() > 1 {
        return Err(ApiError::BadRequest("problem order must be exactly 1 character, A-Z".to_string()));
    }
    for order_char in problem_order.chars(){
        if !order_char.is_alphabetic(){
            return Err(ApiError::BadRequest("problem order must be exactly 1 character, A-Z".to_string()));
        }
    }

    let contest = enyay::get_contest(&state.pool, contest_id, None).await?;
    let contest = match contest{
        Some(existing_contest) => existing_contest,
        None => return Err(ApiError::BadRequest(format!("contest {} does not exist",contest_id)))
    };

    let now = Utc::now();
    if contest.start_time <= now{
        return Err(ApiError::BadRequest("questions must be assigned before a contest begins!".to_string()));
    }

    match enyay::get_problem(&state.pool, problem_id).await?{
        Some(_) => {
            let affected = enyay::assign_contest_problems(&state.pool, contest_id, problem_id, &problem_order).await?;
            if affected == 0{
                return Err(ApiError::BadRequest(
                    "problem order must be unique for each contest".to_string()
                ));
            }
        }
        None => return Err(ApiError::BadRequest(format!("problem {} does not exist!",problem_id)))
    }

    Ok(StatusCode::CREATED)
}

async fn register_contest(
    State(state): State<AppState>,
    Path((contest_id, user_id)): Path<(i64,i64)>
) -> Result<StatusCode, ApiError> {
    match enyay::get_contest(&state.pool, contest_id, Some(user_id)).await?{
        Some(contest) => {
            if Utc::now() < contest.start_time{
                let affected = enyay::register_contest(&state.pool, contest_id, user_id).await?;
                if affected >= 1{
                    return Ok(StatusCode::CREATED);
                } else{
                    return Err(ApiError::BadRequest(
                        format!("user {} already registered",user_id)
                    ));
                }
            } else{
                return Err(ApiError::BadRequest(
                    "you cannot register a contest that has ended or is in progress".to_string()
                ));
            }
        }
        None =>{
            return Err(ApiError::BadRequest(
                format!("contest {} does not exist!",contest_id)
            ))
        }
    }
}

async fn check_registration(
    State(state): State<AppState>,
    Path((contest_id,user_id)): Path<(i64, i64)>
) -> Result<Json<bool>, ApiError>{
    let contest = enyay::get_contest(&state.pool, contest_id, Some(user_id))
    .await?;
    match contest{
        Some(contest) =>{
            return Ok(Json(contest.registered));
        } 
        None => return Err(ApiError::NotFound(format!("contest {contest_id} does not exist")))
    }
}

async fn get_contest_rankings(
    State(state): State<AppState>,
    Path(contest_id): Path<i64>
) -> Result<Json<Vec<enyay::UserRanking>>, ApiError>{
    if enyay::get_contest(&state.pool, contest_id,None).await?.is_none(){
        return Err(ApiError::NotFound(format!("contest {} does not exist",contest_id)));
    }
    return Ok(Json(enyay::get_contest_rankings(&state.pool, contest_id, 20).await?))
}

async fn get_contest_final_ranking(
    State(state): State<AppState>,
    Path(contest_id): Path<i64>
) -> Result<Json<Vec<enyay::FinalRanking>>, ApiError>{
    match enyay::get_contest(&state.pool, contest_id, None).await?{
        None => return Err(ApiError::NotFound(format!("contest {contest_id} does not exist"))),
        Some(contest) =>{
            if Utc::now() < contest.end_time{
                return Err(ApiError::BadRequest(format!("contest {contest_id} is still ongoing")))
            }
        }
    }
    Ok(Json(enyay::get_contest_final_ranking(&state.pool, contest_id, 20).await?))
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

    let language = payload.language.as_deref();

    let id = enyay::insert_submission(
                &state.pool,
                payload.user_id,
                payload.problem_id,
                enyay::Verdict::Pending,
                None,
                None,
                language,
                &payload.source_code,
                None
    )
    .await?;

    Ok((StatusCode::CREATED, Json(IdResponse { id })))
}

async fn create_contest_submission(
    State(state): State<AppState>,
    Path(contest_id): Path<i64>,
    Json(payload): Json<CreateSubmissionRequest>
) -> Result<(StatusCode, Json<IdResponse>), ApiError>{
    let contest_requested = enyay::find_registered_contest(&state.pool, payload.user_id, contest_id).await?;
    let contest = match contest_requested{
        None => return Err(ApiError::BadRequest(format!("you did not register for contest {}",contest_id))),
        Some(contest) => contest
    };

    let now = Utc::now();
    if contest.start_time > now || contest.end_time <= now {
        return Err(ApiError::BadRequest(
            format!("contest {} is not currently active",contest_id)
        ));
    }

    if !enyay::problem_in_contest(&state.pool, contest_id, payload.problem_id).await?{
        return Err(ApiError::BadRequest(
            format!("problem {} is not a part of contest {}", payload.problem_id, contest_id)
        ));
    }
    
    let id = enyay::insert_submission(
        &state.pool, 
        payload.user_id, 
        payload.problem_id, 
        enyay::Verdict::Pending, 
        None, 
        None, 
        payload.language.as_deref(), 
        &payload.source_code, 
        Some(contest_id)
    ).await?;

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
) -> Result<Json<Vec<enyay::SubmissionStatus>>, ApiError> {
    Ok(Json(enyay::get_recent_submissions(&state.pool, 20).await?))
}

async  fn get_recent_submissions_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>
) -> Result<Json<Vec<enyay::SubmissionStatus>>, ApiError> {
    if enyay::get_user(&state.pool, user_id).await?.is_none(){
        return Err(ApiError::NotFound(format!("user {} does not exist",user_id)));
    }
    Ok(Json(enyay::get_recent_submissions_by_user(&state.pool, user_id, 20).await?))
}

async fn get_recent_contest_submissions(
    State(state): State<AppState>,
    Path(contest_id): Path<i64>
) -> Result<Json<Vec<enyay::SubmissionStatus>>, ApiError>{
    if enyay::get_contest(&state.pool, contest_id,None).await?.is_none(){
        return Err(ApiError::NotFound(format!("contest {} does not exist", contest_id)));
    }
    Ok(Json(enyay::get_contest_submissions(&state.pool, contest_id,20).await?))
}

async fn get_recent_user_contest_submission(
    State(state): State<AppState>,
    Path((contest_id,user_id)): Path<(i64,i64)>
) -> Result<Json<Vec<enyay::SubmissionStatus>>, ApiError>{
    if enyay::get_contest(&state.pool, contest_id,Some(user_id)).await?.is_none(){
        return Err(ApiError::NotFound(format!("contest {} does not exist", contest_id)));
    }
    if enyay::get_user(&state.pool, user_id).await?.is_none(){
        return Err(ApiError::NotFound(format!("user {} does not exist",user_id)));
    }
    Ok(Json(enyay::get_contest_user_submissions(&state.pool, contest_id, user_id, 20).await?))
}

//maybe we can use this for a future admin panel to manually rejudge specific submissions
async fn rejudge_submission(
    State(state): State<AppState>,
    Path(submission_id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    match enyay::get_submission(&state.pool, submission_id).await?{
        Some(submission) => {
            if submission.verdict != "PENDING" && submission.verdict != "JUDGING"{
                enyay::update_submission_verdict(
                    &state.pool, 
                    submission_id, 
                    None,
                    None,
                    None,
                    enyay::Verdict::Pending, 
                    None, 
                    None
                )
                .await
                .map_err(|_| ApiError::BadRequest(
                    format!("Failed to rejudge submission {submission_id}. Are you sure it exists?")
                ))?;
            } else{
                return Err(
                    ApiError::BadRequest("You cannot rejudge a submission that is currently being judged!"
                    .to_string()
                ));
            }
        }
        None => return Err(ApiError::NotFound(format!("Submission {} does not exist!",submission_id)))
    }

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
        None,
        None,
        None,
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

fn parse_verdict(value: &str) -> Result<enyay::Verdict, ApiError> {
    enyay::Verdict::from_str(value).map_err(|error| ApiError::BadRequest(error.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    load_env();

    let db_url = std::env::var("DB_URL").unwrap();
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    println!("connecting to database at {db_url}");
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    println!("connected to database");

    let (container_cleanup, cleared) = tokio::join!(
        judge::cleanup_containers(),
        enyay::cleanup_submissions(&pool)
    );
    container_cleanup?;
    let judge_volume = judge::JudgeVolume::new()?;

    match cleared{
        Ok(count) => eprintln!("{} stale submissions restored to pending verdict", count),
        Err(_) => eprintln!("failed to cleanup stale submissions")
    }

    let app_state = AppState{
        pool, 
        judge_volume, 
        judge_limit: Arc::new(Semaphore::new(1))
    };

    let worker_count = 1;
    for _ in 0..worker_count{
        let worker_state = app_state.clone();
        tokio::spawn(async move{
            judge::judge_worker_loop(worker_state).await;
        });
    }

    let contest_pool = app_state.pool.clone();
    tokio::spawn(async move{
        loop {
            if let Err(error) = enyay::manage_contests(contest_pool.clone()).await{
                eprintln!("contest activiation failed: {}", error);
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    let app = Router::new()
        .route("/", get(frontend_index))
        .route("/contests/{contest_id}/problemset/problem/{problem_id}", get(frontend_index))
        .route("/contests/{contest_id}/status/my",get(frontend_index))
        .route("/contests/{contest_id}/status", get(frontend_index))
        .route("/contests/{contest_id}/submit/{problem_id}",get(frontend_index))
        .route("/contests/{contest_id}/registration",get(frontend_index))
        .route("/contests/{contest_id}/home", get(frontend_index))
        .route("/contests/{contest_id}/finalranks",get(frontend_index))
        .route("/contests", get(frontend_index))
        .route("/problemset", get(frontend_index))
        .route("/problemset/problem/{problem_id}", get(frontend_index))
        .route("/submit", get(frontend_index))
        .route("/submit/{problem_id}", get(frontend_index))
        .route("/status", get(frontend_index))
        .route("/status/my",get(frontend_index))
        .route("/login", get(frontend_index))
        .route("/login/users", get(frontend_index))
        .route("/about", get(frontend_index))
        .route("/styles.css", get(frontend_styles))
        .route("/app.js", get(frontend_script))
        .route("/assets/enyayoj-logo.png", get(frontend_logo))
        .route("/assets/enyayoj-mascot.png", get(frontend_mascot))
        .route("/assets/favicon.ico", get(frontend_favicon))
        .route("/health", get(health))
        .route("/users", get(get_users).post(create_user))
        .route("/users/by-name/{user_name}", get(get_user_by_name))
        .route("/users/by-uid/{uid}", get(get_user_by_uid))
        .route("/users/{user_id}", get(get_user))
        .route("/problems", post(create_problem))
        .route("/problems/all/{user_id}", get(get_recent_problems))
        .route("/problems/all", get(get_recent_problems))
        .route("/problems/{problem_id}/{user_id}", get(get_problem_for_user))
        .route("/problems/{problem_id}", get(get_problem))
        .route("/problems/{problem_id}/example",get(get_example_test))
        .route("/problems/{problem_id}/testcases", post(create_testcase))
        .route("/submissions", post(create_submission))
        .route("/submissions/recent", get(get_recent_submissions))
        .route("/submissions/{submission_id}", get(get_submission))
        .route("/submissions/{submission_id}/judge", post(rejudge_submission))
        .route("/submissions/recent/{user_id}", get(get_recent_submissions_by_user))
        .route(
            "/submissions/{submission_id}/verdict",
            patch(update_submission_verdict),
        )
        .route("/contests/{contest_id}/registrations/{user_id}",post(register_contest))
        .route("/contests/problems/{contest_id}/{problem_id}/{problem_order}", post(assign_problem_to_contest))
        .route("/contests/{contest_id}/submissions/recent/{user_id}",get(get_recent_user_contest_submission))
        .route("/contests/{contest_id}/submissions/recent",get(get_recent_contest_submissions))
        .route("/contests/{contest_id}/submissions", post(create_contest_submission))
        .route("/contests/{contest_id}/rankings",get(get_contest_rankings))
        .route("/contests/{contest_id}/inactive/finalrankings",get(get_contest_final_ranking))
        .route("/contests/{contest_id}/problemset/problem/{problem_id}/{user_id}", get(get_contest_problem))
        .route("/contests/{contest_id}/problemset/{user_id}", get(get_user_contest_problems))
        .route("/contests/{contest_id}/problemset", get(get_contest_problems))
        .route("/contests/recent/{user_id}", get(get_recent_user_contests))
        .route("/contests/{contest_id}/check/{user_id}",get(check_registration))
        .route("/contests/create", post(create_contest))
        .route("/contests/recent", get(get_recent_contests))
        .with_state(app_state);

    let addr = bind_addr
        .parse::<SocketAddr>()
        .map_err(|error| ApiError::BadRequest(format!("invalid BIND_ADDR: {error}")))?;
    let listener = TcpListener::bind(addr).await?;

    println!("server listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

fn load_env() {
    if dotenvy::dotenv().is_err() {
        dotenvy::from_filename("backend/.env").ok();
    }
}

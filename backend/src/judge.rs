use tokio::{process::Command,fs,time::{timeout,Duration}};
use std::{fmt::{self}, io, os::unix::process::ExitStatusExt, str::FromStr};
use std::{path::PathBuf, process::{ExitStatus, Output, Stdio}, cmp::max};
use chrono::{DateTime, Utc};
use crate::{AppState, enyay::*};

pub struct JudgeVolume{
    input_volume_mount: String,
    user_volume_mount: String,
    input_dir: PathBuf,
    output_dir:PathBuf
    
}
 
impl JudgeVolume{
    /* 
        Retrieves current dir based on where cargo run is executed. For this, to work
        we need to execute in the backend dir

        can be replaced with an absolute path
     */
    pub fn new() -> io::Result<Self>{
        let whole_dir = std::env::current_dir().expect("Failed to retrieve current dir");
        let output_dir = whole_dir.join("user_inputs");
        let input_dir = whole_dir.join("test_cases");
        std::fs::create_dir_all(&input_dir)?;

        Ok(Self { 
            input_dir: input_dir.to_owned(),
            output_dir: output_dir.to_owned(),
            input_volume_mount: format!("{}:/app/inputs:ro",input_dir.display()),
            user_volume_mount: format!("{}:/app/workspace:rw",output_dir.display())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerError{}

impl fmt::Display for DockerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Docker did not compile/run")
    }
}

impl std::error::Error for DockerError {}
impl From<std::io::Error> for DockerError {
    fn from(_: std::io::Error) -> Self {
        DockerError{}
    }
}
impl From<chrono::ParseError> for DockerError{
    fn from(_:chrono::ParseError) -> Self{
        DockerError {}
    }
}

#[derive(Debug)]
pub struct Metric{
    runtime_ms: Option<i64>,
    peak_memory_kb: Option<i64>,
}

pub struct SubmissionResults{
    pub verdict: Verdict,
    pub metrics: Metric,
}

pub async fn judge_submission(
    submission:&Submission, 
    judge_volume: &JudgeVolume, 
    app_state: &AppState
) -> Result<SubmissionResults,Box<dyn std::error::Error>> {
    let problem = fetch_question(submission, app_state).await?;

    let language = fetch_language(submission).await?;

    let source_code_file = format!("{}_code_submission_{}{}",problem.problem_name,submission.submission_id,language.as_exten());
    let _ = write_out_to_file(&submission.source_code, &judge_volume.output_dir, &source_code_file).await;

    let binary = format!("{}_{}.out",problem.problem_name,submission.submission_id);
    let compile_status = compile_with_docker(&binary, &source_code_file, language, judge_volume).await?;
    

    let submission_results: SubmissionResults;
    if !compile_status.success() {submission_results = SubmissionResults { verdict: Verdict::CompileError, metrics: Metric { runtime_ms: None, peak_memory_kb: None } }}
    else {submission_results = run_tests(submission, &problem, judge_volume, app_state, &binary,&source_code_file, language).await?}
    update_submission_verdict(&app_state.pool, submission.submission_id, submission_results.verdict, submission_results.metrics.runtime_ms, submission_results.metrics.peak_memory_kb).await?;
    let _ = delete_file(&source_code_file, &judge_volume.output_dir).await;
    Ok(submission_results)
}

async fn run_tests(
    submission:&Submission, 
    problem : &Problem,
    judge_volume: &JudgeVolume, 
    app_state: &AppState,
    binary_file: &str,
    source_code: &str,
    language: Language
)-> Result<SubmissionResults,Box<dyn std::error::Error>> {
    let test_cases = get_test_cases(&app_state.pool, problem.problem_id).await?;

    let input_file = format!("{}_input.txt",problem.problem_name);
    let mut verdict = Verdict::Accepted;
    let mut metrics = Metric{runtime_ms: None, peak_memory_kb: None};
    for (i,input) in test_cases.iter().enumerate(){
        let container_name = format!("sub_{}_test_{}_problem_{}", submission.submission_id, i,problem.problem_id);
        let _ = write_out_to_file(&input.input, &judge_volume.input_dir, &input_file).await;
        let user_sol = timeout(
            Duration::from_millis(problem.runtime_ms as u64),
    run_with_docker(problem, binary_file, source_code,&input_file, language, &container_name,judge_volume)
        ).await;
        let submission_results = check_sol(user_sol, input,&container_name, source_code, problem).await?;
        verdict = submission_results.verdict;
        update_metric(& mut metrics, &submission_results.metrics).await;
        kill_container(&container_name).await?;
        if verdict != Verdict::Accepted {
            break;
        }
    }
    let _ = cleanup(&input_file, binary_file, judge_volume).await;
    //when crashes occur due to MLE or TLE, metrics is not calculated properly but we can assume they maxed resource usage
    if verdict == Verdict::MemoryLimitExceeded { metrics.peak_memory_kb = Some(problem.memory_mb*1024); }
    Ok(SubmissionResults { verdict, metrics})

}

async fn check_sol<E>(user_sol:Result<Result<Output,DockerError>,E>, input:&TestCase, container_name: &str, source_code: &str,problem : &Problem) -> Result<SubmissionResults,DockerError>{
    match user_sol {
        Ok(Ok(output)) => {
            let metrics = docker_metrics(&output, container_name).await?;
            match output.status.code() {
                Some(0) => {
                    if String::from_utf8_lossy(&output.stdout).into_owned().trim() == input.solution.trim() {
                        return Ok(SubmissionResults { verdict:Verdict::Accepted, metrics });
                    }
                    Ok(SubmissionResults { verdict:Verdict::WrongAnswer, metrics })
                },
                Some(137) => Ok(SubmissionResults { verdict:Verdict::MemoryLimitExceeded, metrics }),
                _ =>{
                    let _ = extract_runtime_error(&output.stderr,source_code).await;
                    Ok(SubmissionResults { verdict: Verdict::RunTimeError, metrics })
                } 
            }
        },
        Ok(Err(_)) => Ok(SubmissionResults { verdict: Verdict::Pending, metrics: Metric { runtime_ms: None, peak_memory_kb: None} }),
        Err(_) => Ok(SubmissionResults { verdict: Verdict::TimeLimitExceeded, metrics: Metric { runtime_ms: Some(problem.runtime_ms), peak_memory_kb: None } })
    }
}

async fn compile_with_docker(
    compiled_file:&str,
    file_name: &str, 
    language: Language, 
    judge_volume: &JudgeVolume
) -> Result<ExitStatus,DockerError> {
    let command = language.compile_command(file_name, compiled_file);
    if command.is_empty() {
        return Ok(ExitStatus::from_raw(0))
    }
    let compile = Command::new("docker")
        .args(["run","--rm"])
        .args(["-v",&judge_volume.user_volume_mount])
        .args(["--user", "1000:1000"])
        .args(["-w","/app/workspace"])
        .arg(language.as_img())
        .args(command)
        .status()
        .await?;
    Ok(compile)
}

async fn run_with_docker(
    question:&Problem, 
    compiled_file:&str, 
    source_code: &str,
    test_cases:&str,
    language:Language, 
    docker_name: &str,
    judge_volume: &JudgeVolume
) -> Result<Output,DockerError> {
    let memory_limit = &format!("{}m",question.memory_mb.to_string());
    let child = Command::new("docker")
        .args(["run","--name",docker_name])       
        .args(["--memory", memory_limit])       
        .args(["--cpus", "1.0"])           
        .args(["--network", "none"])       
        .args(["-v", &judge_volume.input_volume_mount])
        .args(["-v",&judge_volume.user_volume_mount])
        .args(["--user", "1000:1000"])
        .args(["-w", "/app/workspace"])
        .arg(language.as_img())
        .args(["sh", "-c", &language.run_command(source_code, compiled_file, test_cases)])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let output = child.wait_with_output().await?;
    Ok(output)
}

async fn docker_metrics(
    output: &Output,
    docker_name: &str
) -> Result<Metric, DockerError> {
    let inspect_child = Command::new("docker")
        .args(["inspect", docker_name, "--format", "{{.State.StartedAt}} {{.State.FinishedAt}}"])
        .output()
        .await?;
    let inspect_str = String::from_utf8_lossy(&inspect_child.stdout);
    let timestamps: Vec<&str> = inspect_str.split_whitespace().collect();
    let mut duration_ms = 0;

    if timestamps.len() == 2 {
        let started_at = DateTime::parse_from_rfc3339(timestamps[0])?.with_timezone(&Utc);
        let finished_at = DateTime::parse_from_rfc3339(timestamps[1])?.with_timezone(&Utc);
        duration_ms = finished_at.signed_duration_since(started_at).num_milliseconds();
    }

   let stderr_str = String::from_utf8_lossy(&output.stderr);
    let mut peak_memory_kb = 0;
    
    for line in stderr_str.lines() {
        if line.starts_with("JUDGE_MEM:") {
            let bytes_str = line["JUDGE_MEM:".len()..].trim();
            if let Ok(bytes) = bytes_str.parse::<u64>() {
                peak_memory_kb = bytes as i64 / 1024;
            }
        }
    }

    Ok(Metric { runtime_ms: Some(duration_ms), peak_memory_kb: Some(peak_memory_kb) })
}

async fn extract_runtime_error(raw_stderr: &[u8], _file_name: &str) -> String {

    let stderr_str = String::from_utf8_lossy(raw_stderr);
    

    let mut actual_stderr = String::new();

    //just realized the format only worked for c++
    for line in stderr_str.lines() {
        if !line.starts_with("JUDGE_MEM") && !line.is_empty() {
            println!("{line}");
            if !actual_stderr.is_empty() {
                actual_stderr.push('\n');
            }
            //actual_stderr.push_str(&line[file_name.len()+1..]);
        } 
    }
    //println!("{}",actual_stderr);
    actual_stderr
}

async fn update_metric(old_metric: &mut Metric, new_metric: &Metric){
    if new_metric.runtime_ms.is_some() {
        old_metric.runtime_ms = max(old_metric.runtime_ms, new_metric.runtime_ms);
    }

    if let Some(val) = old_metric.runtime_ms{
        if val < 0 {
            old_metric.runtime_ms = None;
        }
    }
    
    if new_metric.peak_memory_kb.is_some() {
        old_metric.peak_memory_kb = max(old_metric.peak_memory_kb, new_metric.peak_memory_kb);
    }
}

async fn kill_container(docker_name:&str) -> std::io::Result<()>{
    let _ = Command::new("docker")
        .args(["rm", "-f", "-v",docker_name])                                                              
        .output()
        .await?;
    Ok(())
}

async fn write_out_to_file(output: &str, dir: &PathBuf, file_name: &str) -> io::Result<String>{
    let output = output.trim();
    let path =&dir;
    let write_path = path.join(file_name);
    fs::write(write_path,output)
        .await?;
    Ok(file_name.to_string())
}

async fn delete_file(file_name:&str, path: &PathBuf) -> io::Result<()>{
    let path = &path.join(file_name);
    fs::remove_file(path).await?;
    Ok(())
}
async fn cleanup(input_file:&str, binary_file: &str, judge_volume: &JudgeVolume) -> io::Result<()>{
    let _ = delete_file(input_file, &judge_volume.input_dir).await?;
    let _ = delete_file(binary_file, &judge_volume.output_dir).await?;
    Ok(())
}

async fn fetch_question(submission:&Submission, app_state: &AppState) -> Result<Problem,sqlx::Error>{
    let submission_question = get_problem(&app_state.pool, submission.problem_id).await?;
    let problem;
    match submission_question{
        Some(question) => problem = question,
        None => return Err(sqlx::Error::RowNotFound)
    }
    Ok(problem)
}

async fn fetch_language(submission:&Submission) -> Result<Language, LanguageNotSupportedError>{
    let language;
    match &submission.language{
        Some(lang) => language = lang,
        None => return Err(LanguageNotSupportedError)
    }
    Language::from_str(language)
}
#[allow(unused_imports)]
use tokio::{process::Command,io::AsyncWriteExt,fs,time::{timeout,Duration}};
use std::{fmt::{self}, io, str::FromStr};
#[allow(unused_imports)]
use std::{fmt::Error, io::Stdin, path::PathBuf, process::{ExitStatus, Output, Stdio}};
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
pub struct DockerError;

impl fmt::Display for DockerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Docker did not compile/run")
    }
}

impl std::error::Error for DockerError {}

pub async fn judge_submission(
    submission:&Submission, 
    judge_volume: &JudgeVolume, 
    app_state: &AppState
) -> Result<Verdict,Box<dyn std::error::Error>> {
    let problem = fetch_question(submission, app_state).await?;

    let language = fetch_language(submission).await?;

    let source_code_file = format!("{}_code_submission_{}{}",problem.problem_name,submission.submission_id,language.as_exten());
    let _ = write_out_to_file(&submission.source_code, &judge_volume.output_dir, &source_code_file).await;

    let binary = format!("{}_{}.out",problem.problem_name,submission.submission_id);
    let compile_status = compile_with_docker(&binary, &source_code_file, language.as_str(), judge_volume).await?;
    let _ = delete_file(&source_code_file, &judge_volume.output_dir).await;
    if !compile_status.success() {return Ok(Verdict::CompileError)}
    Ok(run_tests(submission, &problem, judge_volume, app_state, &binary, language.as_str()).await?)
}

async fn run_tests(
    submission:&Submission, 
    problem : &Problem,
    judge_volume: &JudgeVolume, 
    app_state: &AppState,
    binary_file: &str,
    compiler: [&str;2]
)-> Result<Verdict,Box<dyn std::error::Error>> {
    let test_cases = get_test_cases(&app_state.pool, problem.problem_id).await?;

    let input_file = format!("{}_input.txt",problem.problem_name);
    let mut correct = true;
    for (i,input) in test_cases.iter().enumerate(){
        let container_name = format!("sub_{}_test_{}", submission.submission_id, i);
        let _ = write_out_to_file(&input.input, &judge_volume.input_dir, &input_file).await;
        let user_sol = timeout(
            Duration::from_millis(problem.runtime_ms as u64),
    run_with_docker(problem, binary_file, &input_file, compiler, &container_name,judge_volume)
        ).await;
        match user_sol {
            Ok(Ok(output)) => {
                match output.status.code() {
                    Some(0) => correct = String::from_utf8_lossy(&output.stdout).into_owned().trim() == input.solution.trim(),
                    Some(137) => {
                        cleanup(&input_file, binary_file, judge_volume).await?;
                        return Ok(Verdict::MemoryLimitExceeded);
                    }
                    _ => {
                        cleanup(&input_file, binary_file, judge_volume).await?;
                        return Ok(Verdict::RunTimeError);
                    }
                }
                if !correct{break;}
            },
            Ok(Err(DockerError)) => return Ok(Verdict::Pending),
            Err(_) => {
                let _ = Command::new("docker")
                .args(["kill",binary_file])
                .output()
                .await;
                cleanup(&input_file, binary_file, judge_volume).await?;
                return Ok(Verdict::TimeLimitExceeded)
            }
        }
    }
    cleanup(&input_file, binary_file, judge_volume).await?;
    if correct {Ok(Verdict::Accepted)}
    else {Ok(Verdict::WrongAnswer)}
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

async fn compile_with_docker(
    compiled_file:&str,
    file_name: &str, 
    compiler: [&str;2], 
    judge_volume: &JudgeVolume
) -> Result<ExitStatus,DockerError> {
    let compile = Command::new("docker")
        .args(["run","--rm"])
        .args(["-v",&judge_volume.user_volume_mount])
        .args(["--user", "1000:1000"])
        .args(["-w","/app/workspace"])
        .arg(compiler[0])
        .args([compiler[1],file_name, "-o", &compiled_file])
        .status()
        .await
        .expect("Failed to compile code");
    Ok(compile)
}

async fn run_with_docker(
    question:&Problem, 
    compiled_file:&str, 
    test_cases:&str,
    compiler:[&str;2], 
    docker_name: &str,
    judge_volume: &JudgeVolume
) -> Result<Output,DockerError> {
    let memory_limit = &format!("{}m",question.memory_mb.to_string());
    let redirect_test = format!(r#"./"{}" < "/app/inputs/{}""#,compiled_file,test_cases);

    let child = Command::new("docker")
        .args(["run","--name",docker_name, "--rm"])       
        .args(["--memory", memory_limit])       
        .args(["--cpus", "1.0"])           
        .args(["--network", "none"])       
        .args(["-v", &judge_volume.input_volume_mount])
        .args(["-v",&judge_volume.user_volume_mount])
        .args(["--user", "1000:1000"])
        .args(["-w", "/app/workspace"])
        .arg(compiler[0])
        .args(["sh", "-c", &redirect_test])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run code");
    let output = child.wait_with_output().await.expect("Fail to read output");
    Ok(output)
}

async fn write_out_to_file(output: &str, dir: &PathBuf, file_name: &str) -> io::Result<String>{
    let output = output.trim();
    let path =&dir;
    let write_path = path.join(file_name);
    fs::write(write_path,output)
        .await?;
    Ok(file_name.to_string())
}


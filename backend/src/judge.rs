#[allow(unused_imports)]
use tokio::{process::Command,io::AsyncWriteExt,fs};
use std::{fmt::{self}, str::FromStr};
#[allow(unused_imports)]
use std::{fmt::Error, io::Stdin, path::PathBuf, process::{ExitStatus, Output, Stdio}};
use crate::{AppState, enyay::*};

pub struct JudgeVolume{
    volume_mount: String,
    input_dir: PathBuf
}
 
impl JudgeVolume{
    /* 
        Retrieves current dir based on where cargo run is executed. For this, to work
        we need to execute in the backend dir

        can be replaced with an absolute path
     */
    pub fn new() -> Self{
        let mut input_dir = std::env::current_dir().expect("Failed to retrieve current dir");
        input_dir = input_dir.join("user_input");
        Self { 
            input_dir: input_dir.to_owned(),
            volume_mount: format!("{}:/app",input_dir.display())
        }
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
) -> Result<(),Box<dyn std::error::Error>> {
    let problem = fetch_question(submission, app_state).await?;

    let language = fetch_language(submission).await?;

    let source_code_file = format!("{}_code_submission_{}{}",problem.problem_name,submission.submission_id,language.as_exten());
    write_out_to_file(&submission.source_code, judge_volume, &source_code_file).await;

    let binary = compile_with_docker(&submission,&problem, &source_code_file, language.as_str(), judge_volume).await?;
    run_tests(submission, &problem, judge_volume, app_state, &binary, language.as_str()).await?;
    let _ = delete_file(&source_code_file, judge_volume).await;
    Ok(())
}

async fn run_tests(
    submission:&Submission, 
    problem : &Problem,
    judge_volume: &JudgeVolume, 
    app_state: &AppState,
    binary_file: &str,
    compiler: [&str;2]
)-> Result<(),Box<dyn std::error::Error>> {
    let test_cases = get_test_cases(&app_state.pool, problem.problem_id).await?;

    let input_file = format!("{}_input.txt",problem.problem_name);
    let solution_file = format!("{}_solution.txt",problem.problem_name);
    for input in test_cases{
        write_out_to_file(&input.input, judge_volume, &input_file).await;
        write_out_to_file(&input.solution,judge_volume,&solution_file).await;
        let user_sol = run_with_docker(submission, problem, binary_file, &input_file, compiler, judge_volume).await?;
        //check answewr
        let _ = delete_file(&input_file, judge_volume).await;
        let _ = delete_file(&solution_file, judge_volume).await;
        let _ = delete_file(&user_sol,judge_volume).await;
    }
    let _ = delete_file(binary_file, judge_volume).await;
    Ok(())
}

async fn delete_file(file_name:&str, judge_volume: &JudgeVolume) -> ExitStatus{
    let rm = Command::new("rm")
        .current_dir(&judge_volume.input_dir)
        .arg(file_name)
        .status()
        .await
        .expect("Failed to delete file");
    println!("{rm}");
    rm
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
    submission:&Submission, 
    question:&Problem, 
    file_name: &str, 
    compiler: [&str;2], 
    judge_volume: &JudgeVolume
) -> Result<String,DockerError> {
    let compiled_file = format!("{}_{}.out",question.problem_name,submission.submission_id);

    let compile = Command::new("docker")
        .args(["run","--rm"])
        .args(["-v",&judge_volume.volume_mount])
        .args(["-w","/app"])
        .arg(compiler[0])
        .args([compiler[1],file_name, "-o", &compiled_file])
        .status()
        .await
        .expect("Failed to compile code");
    if !compile.success() {panic!("Error compiling");}

    Ok(compiled_file)
}

async fn run_with_docker(
    submission:&Submission, 
    question:&Problem, 
    compiled_file:&str, 
    test_cases:&str,
    compiler:[&str;2], 
    judge_volume: &JudgeVolume
) -> Result<String,DockerError> {
    let memory_limit = &format!("{}m",question.memory_mb.to_string());
    let redirect_test = format!(r#"./"{}" < "{}""#,compiled_file,test_cases);

    let child = Command::new("docker")
        .args(["run", "-i", "--rm"])       
        .args(["--memory", memory_limit])       
        .args(["--cpus", "1.0"])           
        .args(["--network", "none"])       
        .args(["-v", &judge_volume.volume_mount])
        .args(["-w", "/app"])
        .arg(compiler[0])
        .args(["sh", "-c", &redirect_test])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run code");
    let output = child.wait_with_output().await.expect("Fail to read output");

    let out_str = String::from_utf8_lossy(&output.stdout);
    let output_file = format!("{}_response_{}.txt",question.problem_name,submission.submission_id);
    write_out_to_file(&out_str, judge_volume,&output_file).await;
    Ok(output_file)
}

async fn write_out_to_file(output: &str, judge_volume: &JudgeVolume, file_name: &str) -> String{
    let output = output.trim();
    let path =&judge_volume.input_dir;
    let write_path = path.join(file_name);
    fs::write(write_path,output)
        .await
        .expect("Failed to write to disk");
    file_name.to_string()
}


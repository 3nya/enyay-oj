#[allow(unused_imports)]
use tokio::{process::Command,io::AsyncWriteExt,fs};
use std::{str::FromStr};
#[allow(unused_imports)]
use std::{fmt::Error, io::Stdin, path::PathBuf, process::{ExitStatus, Output, Stdio}};
use crate::{AppState, enyay::*};

struct JudgeVolume{
    volume_mount: String,
    input_dir: PathBuf
}
 
impl JudgeVolume{
    /* 
        Retrieves current dir based on where cargo run is executed. For this, to work
        we need to execute in the backend dir

        can be replaced with an absolute path
     */
    fn new() -> Self{
        let mut input_dir = std::env::current_dir().expect("Failed to retrieve current dir");
        input_dir = input_dir.join("user_input");
        Self { 
            input_dir: input_dir.to_owned(),
            volume_mount: format!("{}:/app",input_dir.display())
        }
    }
}

pub async fn judge_main(/*app_state: &AppState*/){
    let judge_volumes = JudgeVolume::new();
    let _ = judge_submission(&judge_volumes).await;
    
   /* let test_question = Problem{
        problem_id: 1,
        problem_name: String::from("Add 2"),
        runtime_ms: 1,
        memory_mb: 128
    };

    let language = "c++20";
    let compiler = Language::from_str(language).expect("it should have worked bud");
    let compiler = compiler.as_str();
    let _output = compile_with_docker(&test_question, "AddTwo.cpp", compiler, &judge_volumes).await;*/
}   

async fn judge_submission(/*submission:&Submission,*/ judge_volume: &JudgeVolume, /*(app_state: &AppState*/) -> Result<(),Box<dyn std::error::Error>> {
    //let problem = fetch_question(submission, app_state).await?;
    let problem = Problem{
        problem_id: 1,
        problem_name: String::from("Add 2"),
        runtime_ms: 1,
        memory_mb: 128
    };
    //let language = fetch_language(submission).await?;
    let language = Language::from_str("c++20")?;
    
    /* stored text > file > compile & run file

    let source_code_file = format!("user_code_submission_{}{}",submission.submission_id,language.as_exten());
    write_out_to_file(&submission.source_code, judge_volume, &source_code_file).await;
    */
    let source_code_file = format!("AddTwo{}",language.as_exten());
    compile_with_docker(&problem, &source_code_file, language.as_str(), judge_volume).await;
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



async fn compile_with_docker(question:&Problem, file_name: &str, compiler: [&str;2], judge_volume: &JudgeVolume) -> String {
    let compile = Command::new("docker")
        .args(["run","--rm"])
        .args(["-v",&judge_volume.volume_mount])
        .args(["-w","/app"])
        .arg(compiler[0])
        .args([compiler[1],file_name, "-o", "a.out"])
        .status()
        .await
        .expect("Failed to compile code");
    if !compile.success() {panic!("Error compiling");}

    run_with_docker(question, compiler,judge_volume).await
}

async fn run_with_docker(question:&Problem, compiler:[&str;2], judge_volume: &JudgeVolume) -> String{
    let memory_limit = &format!("{}m",question.memory_mb.to_string());
    let test_cases = "input.txt"; //replace this with a query from db using question_id
    let redirect_test = format!("./a.out < {}",test_cases);

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
    write_out_to_file(&out_str, judge_volume,"user_output.txt").await;
    out_str.into_owned()
}

async fn write_out_to_file(output: &str, judge_volume: &JudgeVolume, file_name: &str){
    let output = output.trim();
    let path =&judge_volume.input_dir;
    let write_path = path.join(file_name);
    fs::write(write_path,output)
        .await
        .expect("Failed to write to disk")
}


use tokio::process::Command;
use std::{fmt::Error, process::{ExitStatus, Stdio}};
use crate::enyay::Problem;

#[tokio::main]
pub async fn main(){
    let mut input_dir = std::env::current_dir().expect("Failed to retrieve current dir");
    input_dir = input_dir.join("user_input");
    
    let test_question = Problem{
        problem_id: 1,
        problem_name: String::from("Add 2"),
        runtime_ms: 1,
        memory_mb: 128
    };

    let volume_mount = format!("{}:/app",input_dir.display());
    //let compile = run_code("Hello.cpp").await.unwrap();
    let _compile = run_with_docker(&test_question, "Hello.cpp", &volume_mount).await;
    //println!("{:?}",compile);
}   

async fn run_with_docker(question:&Problem, file_name: &str, volume_mount: &str) {
    let compile = Command::new("docker")
        .args(["run","--rm"])
        .args(["-v",volume_mount])
        .args(["-w","/app"])
        .arg("gcc:latest")
        .args(["g++",file_name, "-o", "a.out"])
        .status()
        .await
        .expect("Failed to compile code");
    if !compile.success() {panic!("Error compiling");}
    let memory = format!("{}m",question.memory_mb.to_string());
    let mut child = Command::new("docker")
        .args(["run", "-i", "--rm"])       
        .args(["--memory", &memory])       
        .args(["--cpus", "1.0"])           
        .args(["--network", "none"])       
        .args(["-v", &volume_mount])
        .args(["-w", "/app"])
        .arg("gcc:latest")
        .arg("./a.out")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run code");
    drop(child.stdin.take());
    let output = child.wait_with_output().await.expect("Fail to read output");
    println!("Container Exit Status: {}", output.status);
    println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
}
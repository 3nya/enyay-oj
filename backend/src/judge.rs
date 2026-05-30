#[allow(unused_imports)]
use tokio::{process::Command,io::AsyncWriteExt,fs};
#[allow(unused_imports)]
use std::{fmt::Error, io::Stdin, path::PathBuf, process::{ExitStatus, Output, Stdio}, thread::spawn};
use crate::enyay::Problem;

#[tokio::main]
pub async fn main(){
    /* 
        Retrieves current dir based on where cargo run is executed. For this, to work
        we need to execute in the backend dir
     */
    let mut input_dir = std::env::current_dir().expect("Failed to retrieve current dir");
    input_dir = input_dir.join("user_input");
    
    let test_question = Problem{
        problem_id: 1,
        problem_name: String::from("Add 2"),
        runtime_ms: 1,
        memory_mb: 128
    };

    let volume_mount = format!("{}:/app",input_dir.display());
    let output = compile_with_docker(&test_question, "AddTwo.cpp", &volume_mount).await;
    let _write = write_out_to_file(output.trim(),&input_dir).await;
}   

async fn compile_with_docker(question:&Problem, file_name: &str, volume_mount: &str) -> String {
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
    run_with_docker(&memory, volume_mount).await
}

async fn run_with_docker(memory_limit:&str, volume_mount: &str) -> String{
    let child = Command::new("docker")
        .args(["run", "-i", "--rm"])       
        .args(["--memory", memory_limit])       
        .args(["--cpus", "1.0"])           
        .args(["--network", "none"])       
        .args(["-v", volume_mount])
        .args(["-w", "/app"])
        .arg("gcc:latest")
        .args(["sh", "-c", "./a.out < input.txt"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to run code");


    let output = child.wait_with_output().await.expect("Fail to read output");
    let out_str = String::from_utf8_lossy(&output.stdout);
    out_str.into_owned()
    //println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
}

async fn write_out_to_file(output: &str, path: &PathBuf){
    let output = output.trim();
    let write_path = path.join("user_output.txt");
    fs::write(write_path,output)
        .await
        .expect("Failed to write to disk")
}


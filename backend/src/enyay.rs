use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use dotenv::dotenv;
use std::env;

#[allow(dead_code)] 
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let db_url = env::var("DB_URL")?;
    let pool = connect_database(db_url).await
        .expect("Could not connect to database");
    match create_user("Enya", &pool).await {
        Ok(()) => println!("User created"),
        Err(_) => println!("User failed to be created")
    }
    Ok(())
}

async fn connect_database(db_url:String) -> Result<MySqlPool, sqlx::Error>{
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&db_url).await?;
    Ok(pool)
}

async fn create_user(name:&str,pool: &MySqlPool) -> Result<(),sqlx::Error>{
    sqlx::query("INSERT INTO users (user_name) VALUES (?)")
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Problem {
    pub problem_id: i64,
    pub problem_name: String,
    pub runtime_ms: i64,
    pub memory_mb: i64,
}
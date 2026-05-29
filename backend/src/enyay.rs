use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use dotenv::dotenv;
use std::env;

 
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let db_url = env::var("DB_URL")?;
    let pool = connect_database(db_url).await
        .expect("Could not connect to database");
    create_user("Bob", &pool).await.expect("pls bro");
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
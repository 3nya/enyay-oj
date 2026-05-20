use sqlx::{mysql::MySqlPoolOptions};
 
#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect("db url").await?;

    Ok(())

}
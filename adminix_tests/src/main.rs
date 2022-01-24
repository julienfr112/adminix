use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;

adminix_macro::prepare!();

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};

    #[actix_rt::test]
    async fn test_stock() {
        dotenv().ok();
        let db_url = "example.db";
        println!(
            "pwd : {} sburl {}",
            std::env::current_dir().unwrap().display(),
            &db_url
        );
        let pool = SqlitePool::connect(&db_url)
            .await
            .expect("cannot connect to database");
        let mut app = test::init_service(
            App::new()
                .data(pool.clone())
                .service(web::scope("/admin").configure(configure_adminix)),
        )
        .await;
        let req = test::TestRequest::post().uri("/").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("env DATABASE_URL not defiend");
    println!(
        "pwd : {} sburl {}",
        std::env::current_dir().unwrap().display(),
        &db_url
    );
    let pool = SqlitePool::connect(&db_url)
        .await
        .expect("cannot connect to database");
    //sqlx::migrate!().run(&pool).await?;
    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .service(web::scope("/admin").configure(configure_adminix))
    })
    .bind(("localhost", 8080))?
    .run()
    .await?;
    Ok(())
}

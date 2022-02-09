#[cfg(test)]
mod tests {
    use actix_web::dev::ServiceRequest;
    use actix_web::{test, web, App, Error};
    use actix_web_grants::{GrantsMiddleware, PermissionGuard};
    use sqlx::SqlitePool;

    async fn extract(_req: &ServiceRequest) -> Result<Vec<String>, Error> {
        Ok(vec!["admin".to_string()])
    }

    adminix_macro::prepare!();

    #[actix_rt::test]
    async fn test_stock() {
        println!("started");
        let pool = SqlitePool::connect("example.db")
            .await
            .expect("cannot connect to database");
        let mut app = test::init_service(
            App::new()
                .data(pool.clone())
                .wrap(GrantsMiddleware::with_extractor(extract))
                .service(
                    web::scope("/admin")
                        .configure(configure_adminix)
                        .guard(PermissionGuard::new("admin".to_string())),
                ),
        )
        .await;
        let req = test::TestRequest::post().uri("/").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }
}

fn main() {}

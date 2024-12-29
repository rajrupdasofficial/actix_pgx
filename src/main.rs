use actix_web::{web, App, HttpServer};
mod login;
mod signup;

use login::login::login;
use signup::signup::signup;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/signup", web::post().to(signup))
            .route("/login", web::post().to(login))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

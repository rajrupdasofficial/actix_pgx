use actix_web::{web, App, HttpServer};
mod signup;
use signup::signup::signup;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/signup", web::post().to(signup)))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

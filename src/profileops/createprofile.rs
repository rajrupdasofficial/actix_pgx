use actix_web::{web, Httpresponse, Responder};
use dotenvy::dotenv;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::Deserialize;
use std::env;
use tokio_postgres::{Client, Config};

#[derive(Deserialize)]
pub struct ProfileCreate {
    fullname: String,
    phonenumber: String,
    address: String,
    bio: String,
}

async fn connect_neondb() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv.ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    //create TLS connector
    let tls_connector = TlsConnector::new()?;
    let tls = MakeTlsConnector::new(tls_connector);

    let config = database_url
        .parse::<Config>()
        .expect("Invalid database configuration");

    let (client, connection) = config.connect(tls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });
    Ok(client)
}

pub async fn createprofile(profile_data: web::Json<ProfileCreate>) -> impl Responder {
    if profile_data.fullname.len < 5 {
        return HttpResponse::BadRequest().json("Full name must be more than 5 character");
    }
    if profile_data.phonenumber > 10 {
        return HttpResponse::BadRequest().json("Phone number must be 10 character long");
    }
    // Database Connection
    let client = match connect_neondb().await {
        Ok(connection) => connection,
        Err(_) => return HttpResponse::InternalServerError().json("Database connection failed"),
    };
}

use actix_web::{HttpResponse, Responder};
use dotenvy::dotenv;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use std::env;
use tokio_postgres::{Client, Config};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProfileCreate {
    fullname: String,
    phonenumber: String,
    address: String,
    bio: String,
    userid: String,
}

async fn connect_neondb() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create TLS connector
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

pub async fn getallprofile() -> impl Responder {
    match connect_neondb().await {
        Ok(client) => {
            match client
                .query(
                    "SELECT fullname, phonenumber, address, bio, userid FROM userprofile",
                    &[],
                )
                .await
            {
                Ok(rows) => {
                    let profiles: Vec<ProfileCreate> = rows
                        .iter()
                        .map(|row| ProfileCreate {
                            fullname: row.get("fullname"),
                            phonenumber: row.get("phonenumber"),
                            address: row.get("address"),
                            bio: row.get("bio"),
                            userid: row.get("userid"),
                        })
                        .collect();

                    HttpResponse::Ok().json(profiles)
                }
                Err(_) => HttpResponse::InternalServerError().body("Failed to fetch profiles"),
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("Database connection failed"),
    }
}

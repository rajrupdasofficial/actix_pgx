use actix_web::{web, HttpResponse, Responder};
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
    userid: String,
}

async fn connect_neondb() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv().ok();

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

pub async fn createprofilefn(profile_data: web::Json<ProfileCreate>) -> impl Responder {
    if profile_data.fullname.len() < 5 {
        return HttpResponse::BadRequest().json("Full name must be more than 5 character");
    }
    if profile_data.phonenumber.len() != 10 {
        return HttpResponse::BadRequest().json("Phone number must be 10 character long");
    }
    // Database Connection
    let client = match connect_neondb().await {
        Ok(connection) => connection,
        Err(_) => return HttpResponse::InternalServerError().json("Database connection failed"),
    };
    match client
        .execute(
            "INSERT INTO userprofile (fullname, phonenumber, address, bio, userid) VALUES ($1, $2, $3, $4, $5)",
                   &[
                       &profile_data.fullname,
                       &profile_data.phonenumber,
                       &profile_data.address,
                       &profile_data.bio,
                       &profile_data.userid,
                   ],
        )
        .await
    {
        Ok(_) => HttpResponse::Created().json("User profile hasbeen created successfully"),
        Err(e) => {
            eprintln!("Profile creation error:{:?}", e);
            HttpResponse::InternalServerError().json("Registration failed")
        }
    }
}

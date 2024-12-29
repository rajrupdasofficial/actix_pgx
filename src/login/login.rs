use actix_web::{web, HttpResponse, Responder};
use bcrypt::verify;
use chrono::{Duration, Utc};
use dotenvy::dotenv;
use jsonwebtoken::{encode, EncodingKey, Header};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env;
use tokio_postgres::{Client, Config};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct Claims {
    id: String, // Changed to String
    email: String,
    exp: usize, // expiration timestamp
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

    // Connect with TLS
    let (client, connection) = config.connect(tls).await?;

    // Spawn connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    Ok(client)
}

pub async fn login(login_data: web::Json<LoginRequest>) -> impl Responder {
    // Input validation
    if login_data.email.is_empty() || login_data.password.is_empty() {
        return HttpResponse::BadRequest().json("Email and password are required");
    }

    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !email_regex.is_match(&login_data.email) {
        return HttpResponse::BadRequest().json("Invalid email format");
    }

    // Database connection
    let client = match connect_neondb().await {
        Ok(client) => client,
        Err(e) => {
            return HttpResponse::InternalServerError().json(format!("Database error: {}", e))
        }
    };

    // User lookup
    let row = match client
        .query_one(
            "SELECT id::text, email, password FROM users WHERE email = $1", // Cast UUID to text
            &[&login_data.email],
        )
        .await
    {
        Ok(row) => row,
        Err(_) => return HttpResponse::Unauthorized().json("Invalid credentials"),
    };

    // Password verification
    let stored_password: String = row.get("password");
    let is_valid_password = match verify(&login_data.password, &stored_password) {
        Ok(result) => result,
        Err(_) => return HttpResponse::InternalServerError().json("Password verification error"),
    };

    if !is_valid_password {
        return HttpResponse::Unauthorized().json("Invalid credentials");
    }

    // JWT Token Generation
    let user_id: String = row.get("id"); // Now gets as String
    let email: String = row.get("email");

    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("Invalid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        id: user_id.to_string(), // Convert UUID to string
        email: email.clone(),
        exp: expiration,
    };

    let token = match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    ) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().json("Token generation failed"),
    };

    // Successful login response
    HttpResponse::Ok().json(serde_json::json!({
        "token": token,
        "user_id": user_id,
        "email": email
    }))
}

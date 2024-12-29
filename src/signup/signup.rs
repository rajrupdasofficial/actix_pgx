use actix_web::{web, HttpResponse, Responder};
use bcrypt::{hash, DEFAULT_COST};
use dotenvy::dotenv;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use regex::Regex;
use serde::Deserialize;
use std::env;
use tokio_postgres::{Client, Config};

#[derive(Deserialize)]
pub struct SignupRequest {
    name: String,
    email: String,
    password: String,
    confirm_password: String,
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

pub async fn signup(signup_data: web::Json<SignupRequest>) -> impl Responder {
    // Input Validation
    if signup_data.password != signup_data.confirm_password {
        return HttpResponse::BadRequest().json("Passwords do not match");
    }

    if signup_data.password.len() < 8 {
        return HttpResponse::BadRequest().json("Password must be at least 8 characters");
    }

    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
    if !email_regex.is_match(&signup_data.email) {
        return HttpResponse::BadRequest().json("Invalid email format");
    }

    // Password Hashing
    let hashed_password = match hash(&signup_data.password, DEFAULT_COST) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::InternalServerError().json("Password hashing failed"),
    };

    // Database Connection
    let client = match connect_neondb().await {
        Ok(connection) => connection,
        Err(_) => return HttpResponse::InternalServerError().json("Database connection failed"),
    };

    // Check Existing User
    let existing_user_query = match client
        .query(
            "SELECT COUNT(*) FROM users WHERE email = $1",
            &[&signup_data.email],
        )
        .await
    {
        Ok(rows) => rows.first().map(|row| row.get::<_, i64>(0)).unwrap_or(0),
        Err(_) => return HttpResponse::InternalServerError().json("Database query error"),
    };

    if existing_user_query > 0 {
        return HttpResponse::BadRequest().json("Email already registered");
    }

    // User Registration
    match client
        .execute(
            "INSERT INTO users (name, email, password) VALUES ($1, $2, $3)",
            &[&signup_data.name, &signup_data.email, &hashed_password],
        )
        .await
    {
        Ok(_) => HttpResponse::Created().json("User registered successfully"),
        Err(e) => {
            eprintln!("Signup error: {:?}", e);
            HttpResponse::InternalServerError().json("Registration failed")
        }
    }
}

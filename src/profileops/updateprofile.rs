use actix_web::{web, HttpResponse, Responder};
use dotenvy::dotenv;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use std::env;
use tokio_postgres::{types::ToSql, Client, Config};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProfileUpdate {
    fullname: Option<String>,
    phonenumber: Option<String>,
    address: Option<String>,
    bio: Option<String>,
    userid: String,
}

async fn connect_neondb() -> Result<Client, Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let tls_connector = TlsConnector::new()?;
    let tls = MakeTlsConnector::new(tls_connector);
    let config = database_url.parse::<Config>()?;
    let (client, connection) = config.connect(tls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });
    Ok(client)
}

pub async fn updateprofile(profile_data: web::Json<ProfileUpdate>) -> impl Responder {
    // Validate user ID
    if profile_data.userid.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "Invalid user ID"
        }));
    }

    // Check if any fields are being updated
    if profile_data.fullname.is_none()
        && profile_data.phonenumber.is_none()
        && profile_data.address.is_none()
        && profile_data.bio.is_none()
    {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "No update fields provided"
        }));
    }

    // Attempt to connect to the database
    let client = match connect_neondb().await {
        Ok(client) => client,
        Err(_) => return HttpResponse::InternalServerError().body("Database connection failed"),
    };

    // Dynamic query builder with Option handling
    let mut query = "UPDATE userprofile SET ".to_string();
    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
    let mut param_index = 1;

    // Conditionally add fields to update using Option
    if let Some(fullname) = &profile_data.fullname {
        query.push_str(&format!("fullname = ${}, ", param_index));
        params.push(fullname);
        param_index += 1;
    }

    if let Some(phonenumber) = &profile_data.phonenumber {
        query.push_str(&format!("phonenumber = ${}, ", param_index));
        params.push(phonenumber);
        param_index += 1;
    }

    if let Some(address) = &profile_data.address {
        query.push_str(&format!("address = ${}, ", param_index));
        params.push(address);
        param_index += 1;
    }

    if let Some(bio) = &profile_data.bio {
        query.push_str(&format!("bio = ${}, ", param_index));
        params.push(bio);
        param_index += 1;
    }

    // Remove trailing comma and space
    query = query.trim_end_matches(", ").to_string();

    // Add WHERE clause
    query.push_str(&format!(" WHERE userid = ${}", param_index));
    params.push(&profile_data.userid);

    // Execute the update
    match client.execute(&query, &params).await {
        Ok(updated_rows) if updated_rows > 0 => HttpResponse::Ok().json(serde_json::json!({
            "status": "success",
            "message": format!("{} rows updated", updated_rows)
        })),
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({
            "status": "error",
            "message": "No rows updated. User ID might not exist."
        })),
        Err(e) => {
            eprintln!("Update error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": "Failed to update profile"
            }))
        }
    }
}

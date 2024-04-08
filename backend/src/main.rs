use actix_web::{web, App, HttpServer, HttpResponse, middleware};
use actix_cors::Cors;
use dotenv::dotenv;
use std::env;
use actix_web::http::header;

pub mod models;
pub mod handlers;

pub mod gpt3; 
pub mod backendtranslationlogic;
pub mod preprocessing;
pub mod errors;
pub mod auth;
pub mod db;

use mongodb::bson::document::Document;

use crate::db::{init_mongo, init_feedback_collection};
use crate::models::{Feedback, User};
use crate::handlers::{login, register, oauth_callback, github_oauth_callback,change_password_handler, logout, get_user_profile, submit_feedback, delete_account_handler, update_user_profile_handler, test_gpt3_endpoint,translate_code_endpoint,backend_translate_code_handler,preprocess_code_route, request_password_reset, reset_password};

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((header::CONTENT_SECURITY_POLICY, "default-src 'self';"))
        .body("Hello CSP!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::init();

    let mongo_uri = env::var("MONGO_URI").expect("MONGO_URI is not set in .env file");

    let mongo_client = mongodb::Client::with_uri_str(&mongo_uri).await.expect("Failed to connect to MongoDB");
    let mongo_database = mongo_client.database("my_database");

    let mongo_collection = mongo_database.collection::<Document>("some_collection"); // Adjust accordingly
    let feedback_collection = mongo_database.collection::<Feedback>("feedback");
    let user_collection = mongo_database.collection::<User>("users");

    let oauth_config = models::OAuthConfig {
        google_client_id: env::var("GOOGLE_CLIENT_ID").expect("Missing GOOGLE_CLIENT_ID"),
        google_client_secret: env::var("GOOGLE_CLIENT_SECRET").expect("Missing GOOGLE_CLIENT_SECRET"),
        google_redirect_uri: env::var("GOOGLE_REDIRECT_URI").expect("Missing GOOGLE_REDIRECT_URI"),
        github_client_id: env::var("GITHUB_CLIENT_ID").expect("Missing GITHUB_CLIENT_ID"),
        github_client_secret: env::var("GITHUB_CLIENT_SECRET").expect("Missing GITHUB_CLIENT_SECRET"),
        github_redirect_uri: env::var("GITHUB_REDIRECT_URI").expect("Missing GITHUB_REDIRECT_URI"),
    };


    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_methods(vec!["GET", "POST","PUT", "DELETE"])
            .allowed_headers(vec![actix_web::http::header::AUTHORIZATION, actix_web::http::header::ACCEPT, actix_web::http::header::CONTENT_TYPE])
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(mongo_collection.clone()))
            .app_data(web::Data::new(oauth_config.clone()))
            .app_data(web::Data::new(feedback_collection.clone()))
            .app_data(web::Data::new(user_collection.clone()))
            .app_data(web::Data::new(mongo_database.clone()))


            
            .route("/login", web::post().to(login))
            .route("/register", web::post().to(register))
            .route("/oauth_callback", web::get().to(oauth_callback))
            .route("/github_oauth_callback", web::get().to(github_oauth_callback))
            .route("/logout", web::get().to(logout))
            .service(
                web::resource("/api/user/update_profile").route(web::put().to(update_user_profile_handler)),
            )
                   
            .route("/submit_feedback", web::post().to(handlers::submit_feedback))
            .route("/api/test_gpt3", web::get().to(handlers::test_gpt3_endpoint))
            .route("/api/translate_code", web::post().to(handlers::translate_code_endpoint))

            .route("/api/user/update_profile", web::put().to(update_user_profile_handler))
            .route("/api/user/delete", web::delete().to(delete_account_handler))
            .service(
                web::resource("/api/user/change_password")
                .route(web::post().to(change_password_handler)),
            )
            .service(handlers::feedback::get_feedback)
            .service(web::resource("/preprocess_code").route(web::post().to(preprocess_code_route)))
            .service(
                web::resource("/backendtranslationlogic").route(web::post().to(backend_translate_code_handler)),
            )
            .service(web::resource("/request-password-reset").route(web::post().to(handlers::request_password_reset)))
            .service(web::resource("/reset-password").route(web::post().to(handlers::reset_password)))
            
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

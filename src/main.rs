use actix_web::{App, HttpResponse, HttpServer, Responder, get, middleware::Logger, post, web};
use askama::Template;
use log::error;
use nanoid::nanoid;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct CreateLinkRequest {
    path: String,
    target: String,
}

struct Link {
    id: String,
    path: String,
    target: String,
    created_at: String,
}

#[get("/go/{path}")]
async fn redirect(path: web::Path<String>, data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let path = path.into_inner();

    // Skip reserved paths
    if path == "links" || path.starts_with("api/") {
        return HttpResponse::NotFound().finish();
    }

    match conn
        .query_row("SELECT target FROM links WHERE path = ?1", [&path], |row| {
            row.get::<_, String>(0)
        })
        .optional()
    {
        Ok(Some(target)) => HttpResponse::Found()
            .append_header(("Location", target))
            .finish(),
        Ok(None) => HttpResponse::NotFound().body("Link not found"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[get("/go/links")]
async fn management_ui(data: web::Data<AppState>) -> impl Responder {
    let conn = data.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, path, target, created_at FROM links")
        .unwrap();
    let links = stmt
        .query_map([], |row| {
            Ok(Link {
                id: row.get(0)?,
                path: row.get(1)?,
                target: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let template = IndexTemplate { links };
    HttpResponse::Ok()
        .content_type("text/html")
        .body(template.render().unwrap())
}

#[post("/go/api/links")]
async fn create_link(
    req: web::Json<CreateLinkRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let conn = data.db.lock().unwrap();

    // Validate input
    if req.path.is_empty() || req.target.is_empty() {
        return HttpResponse::BadRequest().body("Missing parameters");
    }

    let id = nanoid!(8);

    match conn.execute(
        "INSERT INTO links (id, path, target) VALUES (?1, ?2, ?3)",
        [&id, &req.path, &req.target],
    ) {
        Ok(_) => HttpResponse::Created().finish(),
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint") {
                HttpResponse::Conflict().body("Path already exists")
            } else {
                error!("Database error: {}", e);
                HttpResponse::InternalServerError().finish()
            }
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe { std::env::set_var("RUST_LOG", "info") };
    env_logger::init();

    // Initialize DB
    let conn = Connection::open("go-links.db").unwrap();
    conn.execute_batch(include_str!("schema.sql")).unwrap();

    let state = AppState {
        db: Arc::new(Mutex::new(conn)),
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .service(management_ui)
            .service(create_link)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

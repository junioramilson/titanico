use std::time::Duration;

use actix_web::{
    http::{header::ContentType, StatusCode},
    post, web, App, HttpResponse, HttpServer, Responder,
};
use dotenv;
use mongodb::{
    bson::Document,
    options::{ClientOptions, FindOptions, ResolverConfig, UpdateModifications},
    Client, Collection,
};
use serde_json::{json, Value};

#[derive(serde::Deserialize, serde::Serialize)]
struct MongoDbCommand {
    operation: String, // TODO: Use enum
    collection: String,
    database: String,
    filter: Option<Document>,
    update: Option<UpdateModifications>,
    options: Option<serde_json::Value>,
    document: Option<serde_json::Value>,
}

#[post("/command")]
async fn command(
    mongo_client: web::Data<Client>,
    req_body: web::Json<MongoDbCommand>,
) -> impl Responder {
    let perf_start = std::time::Instant::now();
    let collection: Collection<serde_json::Value> = mongo_client
        .database(&req_body.database)
        .collection(&req_body.collection);

    let document = req_body.document.clone().unwrap_or(serde_json::Value::Null);
    let options = req_body.options.clone().unwrap_or(serde_json::Value::Null);

    let mut command_perf_total = Duration::new(0, 0);

    let response = match req_body.operation.as_str() {
        "insertOne" => {
            let mut doc = document.as_object().unwrap().clone();

            doc.insert(
                "createdAt".to_string(),
                serde_json::Value::from(chrono::Utc::now().to_string()),
            );

            doc.insert(
                "updatedAt".to_string(),
                serde_json::Value::from(chrono::Utc::now().to_string()),
            );

            let perf_start = std::time::Instant::now();
            let result = collection
                .insert_one(serde_json::to_value(doc).unwrap(), None)
                .await
                .unwrap();
            let perf_end = std::time::Instant::now();

            command_perf_total = perf_end.duration_since(perf_start);

            HttpResponse::Ok()
                .status(StatusCode::CREATED)
                .insert_header(ContentType::json())
                .body(
                    serde_json::to_string(&json!({
                        "content": result
                    }))
                    .unwrap()
                    .to_string(),
                )
        }
        "findOne" => {
            let perf_start = std::time::Instant::now();
            let result = collection
                .find_one(req_body.filter.clone(), None)
                .await
                .unwrap();
            let perf_end = std::time::Instant::now();

            command_perf_total = perf_end.duration_since(perf_start);

            HttpResponse::Ok().insert_header(ContentType::json()).body(
                serde_json::to_string(&json!({
                    "content": result
                }))
                .unwrap()
                .to_string(),
            )
        }
        "find" => {
            let limit = options
                .get("limit")
                .unwrap_or(&serde_json::Value::from(100))
                .as_i64()
                .unwrap_or(100);

            let perf_start = std::time::Instant::now();
            let mut cursor = collection
                .find(
                    req_body.filter.clone(),
                    FindOptions::builder().limit(limit).build(),
                )
                .await
                .unwrap();

            let mut list = Vec::<Value>::new();

            while cursor.advance().await.unwrap() {
                list.push(serde_json::to_value(&cursor.current()).unwrap());
            }
            let perf_end = std::time::Instant::now();
            command_perf_total = perf_end.duration_since(perf_start);

            HttpResponse::Ok().insert_header(ContentType::json()).body(
                serde_json::to_string(&json!({
                    "content": list
                }))
                .unwrap()
                .to_string(),
            )
        }
        "findAndModify" => {
            let filter = req_body.filter.clone().unwrap_or(Document::new());

            if req_body.update.is_none() {
                return HttpResponse::BadRequest()
                    .insert_header(ContentType::json())
                    .body(
                        serde_json::to_string(&json!({
                            "message": "update field is required"
                        }))
                        .unwrap()
                        .to_string(),
                    );
            }

            let update = req_body.update.clone().unwrap();

            let perf_start = std::time::Instant::now();
            let result = collection
                .find_one_and_update(filter, update, None)
                .await
                .unwrap();
            let perf_end = std::time::Instant::now();

            command_perf_total = perf_end.duration_since(perf_start);

            HttpResponse::Ok().insert_header(ContentType::json()).body(
                serde_json::to_string(&json!({
                    "content": result
                }))
                .unwrap()
                .to_string(),
            )
        }
        _ => HttpResponse::NotFound()
            .insert_header(ContentType::json())
            .body(
                serde_json::to_string(&json!({
                    "message": "Invalid operation"
                }))
                .unwrap()
                .to_string(),
            ),
    };

    let perf_end = std::time::Instant::now();

    let perf_duration = perf_end.duration_since(perf_start);

    let overhead = perf_duration - command_perf_total;

    println!(
        "Operation: {}, Database: {}, Collection: {}, Duration: {:?}, Overhead: {:?}",
        req_body.operation, req_body.database, req_body.collection, command_perf_total, overhead
    );

    response
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    println!("Connecting to MongoDB");
    let mongo_uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set");
    let mut client_options =
        ClientOptions::parse_with_resolver_config(mongo_uri, ResolverConfig::cloudflare())
            .await
            .unwrap();

    client_options.app_name = Some("Titanico Instance".to_string());
    client_options.min_pool_size = Some(10); // TODO: Use env var
    client_options.max_pool_size = Some(500); // TODO: Use env var

    let client = Client::with_options(client_options).unwrap();

    println!("Starting server");

    HttpServer::new(move || {
        App::new()
            .service(command)
            .app_data(web::Data::new(client.clone()))
    })
    .bind(("127.0.0.1", 8080))? // TODO: Use env var
    .run()
    .await
}

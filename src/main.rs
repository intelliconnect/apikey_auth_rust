use actix_web::{dev::ServiceRequest, web, App, HttpMessage, HttpResponse, HttpServer};
use rand::prelude::*;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

mod errors;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let rclient =
            redis::Client::open("redis://127.0.0.1").expect(" could not get redis client ");
        let api_key_auth = actix_web_httpauth::middleware::HttpAuthentication::with_fn(validator);
        let cors = actix_cors::Cors::default().allow_any_origin();

        App::new()
            .app_data(web::Data::new(rclient))
            .wrap(cors)
            .route("/", web::get().to(index))
            .route("/create", web::post().to(create_api_key))
            .service(
                web::scope("/api/{key}")
                    .wrap(api_key_auth)
                    .route("/details", web::get().to(get_details)),
            )
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Details {
    org: String,
    auth_level: usize,
}

// when sending a request to any route under auth middleware send a dummy bearer authentication token
async fn validator(
    req: ServiceRequest,
    _: actix_web_httpauth::extractors::bearer::BearerAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    // get api key from url path
    let key = req
        .match_info()
        .get("key")
        .unwrap()
        .trim()
        .parse::<i64>()
        .map_err(|_| actix_web::error::ErrorBadRequest("api key is not 10 digit no."))?;
    println!("{}", &key);
    // get connection to the redis database
    let mut conn = req
        .app_data::<web::Data<redis::Client>>()
        .unwrap()
        .get_async_connection()
        .await
        .map_err(|_| {
            actix_web::error::ErrorInternalServerError("error getting redis connection")
        })?;
    // get data from redis database
    match (conn.hget(key, "org").await, conn.hget(key, "level").await) {
        (Ok(org), Ok(auth_level)) => {
            // attach the data to the request
            req.extensions_mut().insert(Details { org, auth_level });
            Ok(req)
        }
        (_, _) => Err(actix_web::error::ErrorUnauthorized("api key is invalid")),
    }
}
// /
async fn index() -> HttpResponse {
    HttpResponse::Ok().body("goto /create to create a api key")
}
// /create
async fn create_api_key(
    rclient: web::Data<redis::Client>,
    details: web::Json<Details>,
) -> Result<HttpResponse, errors::Myerror> {
    let mut ring = rand::thread_rng();
    // generate a random 10 digit key
    let mut key: i64 = ring.gen_range(1000000000..9999999999);
    let mut conn = rclient.get_async_connection().await?;
    let mut exists = conn.exists(key).await?;
    // if the key already exists generate a new one
    while exists {
        key = ring.gen_range(1000000000..9999999999);
        exists = conn.exists(key).await?;
    }
    let details = details.into_inner();
    // set key value in redis
    redis::pipe()
        .cmd("HSET")
        .arg(key)
        .arg("org")
        .arg(details.org)
        .ignore()
        .cmd("HSET")
        .arg(key)
        .arg("level")
        .arg(details.auth_level.to_string())
        .query_async(&mut conn)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "api_key": key })))
}

// /api/{api_key}/details
async fn get_details(
    _rclient: web::Data<redis::Client>,
    details: web::ReqData<Details>,
) -> Result<HttpResponse, errors::Myerror> {
    let details = details.into_inner();
    Ok(HttpResponse::Ok().json(details))
}

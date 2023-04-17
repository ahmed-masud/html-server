use actix_web::{get, web, App, HttpServer, Responder};
use serde::Deserialize;
use std::env;
use std::fs;
use std::sync::Mutex;

#[derive(Deserialize)]
struct MetaInfo {
    port: u16,
    user: String,
    level: String,
}

#[get("/{userid}/{endpoint}")]
async fn api_endpoint(
    userid: web::Path<String>,
    endpoint: web::Path<String>,
    state: web::Data<Mutex<Vec<(String, String, String)>>>,
) -> impl Responder {
    let user_id = userid.into_inner();
    let end_point = endpoint.into_inner();

    let state = state.lock().unwrap();

    let response = state
        .iter()
        .find(|(path, _, _)| path == &format!("/{}/{}", user_id, end_point))
        .map(|(_, level, content)| (level.to_string(), content.to_string()))
        .unwrap_or(("Not Found".to_string(), "Not Found".to_string()));

    let (level, content) = response;
    let user_level = state
        .iter()
        .find(|(_, level, _)| level == &level.to_string())
        .map(|(_, _, user)| user.to_string())
        .unwrap_or("Not Found".to_string());

    format!("User ID: {}, Endpoint: {}, Content: {}", user_id, end_point, user_level)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Usage: {} <path_to_directory>", args[0]);
    }
    let dir = &args[1];

    let file_path = format!("{}/index.html", dir);
    let html_document = fs::read_to_string(file_path).expect("Unable to read index.html");

    let (meta_info, endpoints) = parse_meta_info_and_endpoints(&html_document);
    let server_port = meta_info.port;

    let state = web::Data::new(Mutex::new(endpoints));

    HttpServer::new(move || App::new().app_data(state.clone()).service(api_endpoint))
        .bind(("127.0.0.1", server_port))?
        .run()
        .await
}

fn parse_meta_info_and_endpoints(html: &str) -> (MetaInfo, Vec<(String, String, String)>) {
    // Parse meta info
    let port_tag = r#"<meta name="port" content=""#;
    let start_index = html.find(port_tag).unwrap() + port_tag.len();
    let end_index = html[start_index..].find(r#""#).unwrap() + start_index;
    let port_str = &html[start_index..end_index];
    let port = port_str.parse::<u16>().unwrap();

    let user_tag = r#"<meta name="user" value=""#;
    let user_level_tag = r#"level=""#;
    let user_start_index = html.find(user_tag).unwrap() + user_tag.len();
    let user_end_index = html[user_start_index..].find(r#""#).unwrap() + user_start_index;
    let user_str = &html[user_start_index..user_end_index];

    let level_start_index = html.find(user_level_tag).unwrap() + user_level_tag.len();
    let level_end_index = html[level_start_index..].find(r#""#).unwrap() + level_start_index;
    let level_str = &html[level_start_index..level_end_index];

    let meta_info = MetaInfo {
        port,
        user: user_str.to_string(),
        level: level_str.to_string(),
    };

    // Parse endpoints
    let mut endpoints = Vec::new();

    let a_start_tag = "<a href=\"";
    let mut index = 0;

    while let Some(start) = html[index..].find(a_start_tag) {
        let start = start + index + a_start_tag.len();
        let end = html[start..].find("\">").unwrap() + start;
        let path = &html[start..end];

        let content_start = html[end..].find(">").unwrap() + end + 1;
        let content_end = html[content_start..].find("</a>").unwrap() + content_start;

        for i in 1..=6 {
            let level_tag = format!("<H{}>", i);
            let closing_tag = format!("</H{}>", i);

            if let Some(level_start) = html[content_start..content_end].find(&level_tag) {
                let level_start = level_start + content_start + level_tag.len();
                let level_end = html[level_start..content_end].find(&closing_tag).unwrap() + level_start;
                let content = &html[level_start..level_end];
                let level = format!("h{}", i);

                endpoints.push((path.to_string(), level, content.to_string()));
            }
        }

        index = content_end;
    }

    (meta_info, endpoints)
}


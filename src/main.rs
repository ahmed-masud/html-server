#[macro_use]
extern crate rocket;

use rocket::http::ContentType;
use rocket::State;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::fs;
use std::sync::Mutex;
use rocket::response;

// type UserData = Mutex<HashMap<String, (String, Vec<(String, String)>)>>;

type UserData = Mutex<HashMap<String, User>>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
enum ContentLevel {
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl From<usize> for ContentLevel {
    fn from(value: usize) -> Self {
        match value {
            1 => ContentLevel::H1,
            2 => ContentLevel::H2,
            3 => ContentLevel::H3,
            4 => ContentLevel::H4,
            5 => ContentLevel::H5,
            _ => ContentLevel::H6,
        }
    }
}



#[derive(Debug, Clone)]
struct EndpointContent {
    level: ContentLevel,
    content: String,
}
#[derive(Debug, Clone)]
struct User {
    level: ContentLevel,
    endpoints: HashMap<String, Vec<EndpointContent>>,
}


#[get("/<userid>/<endpoint>")]
fn api_endpoint(userid: String, endpoint: String, users: &State<UserData>) -> (ContentType, String) {
    let users = users.lock().unwrap();
    let Some(user) = users.get(&userid) else {
        return (ContentType::HTML, "This user does not exist or the endpoint is not available.".to_string());
    };

    let Some(endpoint_contents) = user.endpoints.get(&endpoint) else {
        return (ContentType::HTML, "Endpoint not found.".to_string());
    };

    eprintln!("{:?}", endpoint_contents);

    // let Some(content) = endpoint_contents
    //     .iter()
    //     .find(|content| &content.level == &user.level) else {
    //         return (ContentType::HTML, "Not Found or insufficient access level.".to_string());
    // };
    let accessible_content: Vec<String> = endpoint_contents
        .iter()
        // yes THIS is backwards
        .filter(|content| content.level >= user.level)
        .map(|content| content.content.clone())
        .collect();

    if accessible_content.is_empty() {
        return (ContentType::HTML, "Not Found or insufficient access level.".to_string());
    }

    eprintln!("{:?}", accessible_content);
    let content = accessible_content.join("\n");
    (ContentType::HTML, content)
}





#[derive(Debug)]
struct HtmlConfig {
    address: String,
    port: u16,
    users: HashMap<String, User>,
}

// 


fn parse_index_html(html: &str) -> HtmlConfig {
    let document = Html::parse_document(html);

    let address = document.select(&Selector::parse(r#"meta[name="address"]"#).unwrap())
        .next()
        .and_then(|e| e.value().attr("content"))
        .unwrap_or("127.0.0.1")
        .to_string();

    let port = document.select(&Selector::parse(r#"meta[name="port"]"#).unwrap())
        .next()
        .and_then(|e| e.value().attr("content"))
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);

    let mut users: HashMap<String, User> = HashMap::new();
    for (i, selector) in (1..=6).map(|i| Selector::parse(&format!("meta[name=\"user\"][level=\"h{}\"]", i)).unwrap()).enumerate() {
        for element in document.select(&selector) {
            let userid = element.value().attr("value").unwrap().to_string();
            let level = ContentLevel::from(i + 1);
            users.insert(userid, User { level, endpoints: HashMap::new() });
        }
    }

    let a_selector = Selector::parse("a[href]").unwrap();
    let h_selectors: Vec<(ContentLevel, Selector)> = (1..=6)
        .map(|i| (ContentLevel::from(i), Selector::parse(&format!("H{}", i)).unwrap()))
        .collect();

    // for a_element in document.select(&a_selector) {
    //     let href = a_element.value().attr("href").unwrap();
    //     let mut endpoint_contents = vec![];

    //     for (level, h_selector) in h_selectors.iter() {
    //         if let Some(h_element) = a_element.select(h_selector).next() {
    //             endpoint_contents.push(EndpointContent {
    //                 level: level.clone(),
    //                 content: h_element.inner_html(),
    //             });
    //         }
    //     }

    //     for user in users.values_mut() {
    //         user.endpoints.insert(href.to_string(), endpoint_contents.clone());
    //     }
    // }
    for a_element in document.select(&a_selector) {
        let href = a_element.value().attr("href").unwrap();
    
        for (level, h_selector) in h_selectors.iter() {
            if let Some(h_element) = a_element.select(h_selector).next() {
                let endpoint_content = EndpointContent {
                    level: level.clone(),
                    content: h_element.inner_html(),
                };
    
                for user in users.values_mut() {
                    if user.level <= level.clone() {
                        user.endpoints.entry(href.to_string()).or_insert_with(Vec::new).push(endpoint_content.clone());
                    }
                }
            }
        }
    }

    eprintln!("{:?}", users);
    HtmlConfig {
        address,
        port,
        users,
    }
}


#[rocket::launch]
fn rocket() -> _ {
    let index_html = fs::read_to_string("index.html").expect("Unable to read index.html");
    let config = parse_index_html(&index_html);

    let figment = rocket::Config::figment()
        .merge(("port", config.port))
        .merge(("address", config.address));

    rocket::custom(figment)
        .manage(Mutex::new(config.users))
        .mount("/", routes![api_endpoint])
}


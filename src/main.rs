use std::net::ToSocketAddrs;
use reqwest::Error;
use url::Url;
use serde_json;
use structopt::StructOpt;

fn main() -> Result<(), Error> {
    #[derive(StructOpt, Debug)]
    #[structopt(name = "curl")]
    struct Opt {
        #[structopt(name = "URL")]
        url: String,

        #[structopt(short = "X", long = "request", default_value = "GET")]
        method: String,

        #[structopt(short = "d", long = "data")]
        data: Option<String>,

        #[structopt(long = "json")]
        json: Option<String>,
    }

    let opt = Opt::from_args();

    let url = &opt.url;
    let method = if opt.json.is_some() { "POST" } else { &opt.method };
    let data = opt.data.as_deref();
    let json: Option<&str> = opt.json.as_deref();

    println!("Requesting URL: {}", url);
    println!("Method: {}", method);
    if let Some(data) = data {
        println!("Data: {}", data);
    }
    if let Some(json_data) = json {
        println!("JSON: {}", json_data);
    }

    if (!url.starts_with("http://")) && (!url.starts_with("https://")) {
        println!("Error: The URL does not have a valid base protocol.");
        std::process::exit(1);
    }

    let parsed_url = match Url::parse(url) {
        Ok(url) => url,
        Err(e) => {
            if e.to_string().contains("invalid IPv6 address") {
                println!("Error: The URL contains an invalid IPv6 address.");
            } else if e.to_string().contains("invalid IPv4 address") {
                println!("Error: The URL contains an invalid IPv4 address.");
            } else if e.to_string().contains("invalid port number") {
                println!("Error: The URL contains an invalid port number.");
            } else {
                println!("Error: {}", e);
            }
            std::process::exit(1);
        }
    };

    if let Some(host) = parsed_url.host_str() {
        let port = parsed_url.port().unwrap_or(80);
        let addr = format!("{}:{}", host, port);
        if addr.to_socket_addrs().is_err() {
            println!("Error: Unable to connect to the server. Perhaps the network is offline or the server hostname cannot be resolved.");
            std::process::exit(1);
        }
    }

    let client = reqwest::blocking::Client::new();
    let response = if let Some(json_data) = json {
        match serde_json::from_str::<serde_json::Value>(json_data) {
            Ok(_) => (),
            Err(e) => {
                panic!("Invalid JSON: Error(\"{}\")", e);
            }
        }
        client.post(url)
            .header("Content-Type", "application/json")
            .body(json_data.to_string())
            .send()?
    } else if method == "POST" {
        if let Some(data) = data {
            let mut data_to_post: Vec<(&str, &str)> = Vec::new();
            for pair in data.split("&") {
                let mut key_value = pair.split("=");
                let key = key_value.next().unwrap();
                let value = key_value.next().unwrap();
                data_to_post.push((key, value));
            }
            client.post(url).form(&data_to_post).send()?
        } else {
            println!("Error: POST method requires data to be specified with -d.");
            std::process::exit(1);
        }
    } else {
        client.get(url).send()?
    };

    if !response.status().is_success() {
        println!("Error: Request failed with status code: {}.", response.status().as_u16());
        std::process::exit(1);
    }

    let body: String = response.text()?;
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        let mut sorted_json = serde_json::Map::new();
        if let serde_json::Value::Object(map) = json {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_json.insert(key.clone(), map[key].clone());
            }
        }
        let pretty_json = serde_json::to_string_pretty(&sorted_json).map_err(|e| {
            println!("Error: Failed to format JSON: {}", e);
            std::process::exit(1);
        })?;
        println!("Response body (JSON with sorted keys):\n{}", pretty_json);
    } else {
        let trimmed_body = body.trim_end();
        println!("Response body:\n{}", trimmed_body);
    }

    Ok(())
}

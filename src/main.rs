use base64::{engine::general_purpose, Engine};
use serde::Deserialize;

#[derive(Deserialize)]
struct Response {
    current: Current,
    daily: Daily,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    weather_code: u32,
    time: String,
}

#[derive(Deserialize)]
struct Daily {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
}

#[derive(Deserialize)]
struct GeoResult {
    results: Option<Vec<GeoLocation>>,
}

#[derive(Deserialize)]
struct GeoLocation {
    name: String,
    latitude: f64,
    longitude: f64,
    timezone: String,
}

fn wmo(code: u32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 | 77 => "Snow",
        80 | 81 | 82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorm",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown",
    }
}

/// Extract city name from timezone string (e.g. "Asia/Keelung" -> "Keelung")
fn city_from_timezone(timezone: &str) -> &str {
    timezone.split('/').last().unwrap_or(timezone)
}

/// Resolve (lat, lon, city_name, iana_timezone) from timezone using Open-Meteo Geocoding API
fn resolve_location(timezone: &str) -> (f64, f64, String, String) {
    let city = city_from_timezone(timezone).replace('_', " ");
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        urlencoding::encode(&city)
    );
    let geo: GeoResult = reqwest::blocking::get(&url)
        .expect("geocoding request failed")
        .json()
        .expect("geocoding parse failed");

    match geo.results.and_then(|r| r.into_iter().next()) {
        Some(loc) => (loc.latitude, loc.longitude, loc.name, loc.timezone),
        None => {
            eprintln!("Could not resolve location for timezone: {}", timezone);
            std::process::exit(1);
        }
    }
}

fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .skip_while(|a| a.as_str() != flag)
        .nth(1)
        .cloned()
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn generate_image(prompt: &str, output: &str, model: &str, resolution: &str) {
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY not set");
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let body = serde_json::json!({
        "contents": [{"role": "user", "parts": [{"text": prompt}]}],
        "generationConfig": {
            "responseModalities": ["TEXT", "IMAGE"],
            "imageConfig": {"imageSize": resolution}
        }
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .expect("failed to build client");
    let resp: serde_json::Value = client
        .post(&url)
        .json(&body)
        .send()
        .expect("Gemini request failed")
        .json()
        .expect("Gemini response parse failed");

    let parts = resp["candidates"][0]["content"]["parts"]
        .as_array()
        .expect("no parts in response");

    for part in parts {
        if let Some(data) = part["inlineData"]["data"].as_str() {
            let bytes = general_purpose::STANDARD
                .decode(data)
                .expect("base64 decode failed");
            std::fs::write(output, bytes).expect("failed to write image");
            println!("Image saved to {}", output);
            return;
        }
    }
    eprintln!("No image returned from Gemini");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if has_flag(&args, "--help") || has_flag(&args, "-h") {
        println!("Usage: weather [OPTIONS]

Options:
  --timezone <tz>     Any IANA timezone (default: America/New_York)
  --image             Generate weather image via Gemini API
  --model <id>        Gemini model ID (default: gemini-3.1-flash-image-preview)
  --prompt <text>     Extra prompt appended to image generation
  --output <path>     Image output path (default: output.png)
  --resolution <res>  Image resolution: 1K, 2K, 4K (default: 1K)
  -h, --help          Show this help

Examples:
  weather --timezone Asia/Taipei
  weather --timezone Asia/Keelung --image --prompt \"dreamy illustration\"

Environment variables:
  GEMINI_API_KEY      Required for --image");
        return;
    }

    let timezone = parse_arg(&args, "--timezone")
        .unwrap_or_else(|| "America/New_York".to_string());
    let image = has_flag(&args, "--image");
    let model = parse_arg(&args, "--model")
        .unwrap_or_else(|| "gemini-3.1-flash-image-preview".to_string());
    let user_prompt = parse_arg(&args, "--prompt");
    let output = parse_arg(&args, "--output")
        .unwrap_or_else(|| "output.png".to_string());
    let resolution = parse_arg(&args, "--resolution")
        .unwrap_or_else(|| "1K".to_string());

    let (lat, lon, city, iana_tz) = resolve_location(&timezone);

    let url = format!(
        "https://api.open-meteo.com/v1/forecast\
?latitude={lat}&longitude={lon}\
&current=temperature_2m,weather_code\
&daily=temperature_2m_max,temperature_2m_min\
&timezone={tz}",
        lat = lat,
        lon = lon,
        tz = urlencoding::encode(&iana_tz),
    );

    let resp: Response = reqwest::blocking::get(&url)
        .expect("request failed")
        .json()
        .expect("parse failed");

    let weather = wmo(resp.current.weather_code);
    let current = resp.current.temperature_2m;
    let max = resp.daily.temperature_2m_max[0];
    let min = resp.daily.temperature_2m_min[0];

    println!(
        "weather={} current={:.1}°C max={:.1}°C min={:.1}°C localtime={}",
        weather, current, max, min, resp.current.time
    );

    if image {
        let mut prompt = format!(
            "{city}, {}, {weather}, current {current:.1}°C, max {max:.1}°C, min {min:.1}°C",
            resp.current.time,
        );
        if let Some(extra) = user_prompt {
            prompt.push_str(". ");
            prompt.push_str(&extra);
        }
        eprintln!("Generating image: {}", prompt);
        generate_image(&prompt, &output, &model, &resolution);
    }
}

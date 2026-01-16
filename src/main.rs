use chrono::Datelike;
mod weather_structs;

use crate::weather_structs::{
    FavoriteLocation, FavoriteWeatherResponse, ForecastDay, ForecastHour, ForecastResponse,
};
use chrono::NaiveDate;
use slint::SharedString;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const API_KEY: &str = "8CYFEA2MNNBQD7YG5C6YDKB5K";

// AM PUS UNDERLINE PT CA ESTE UN STRUCT PE CARE NU IL CONSTRUIESC EFECTIV, IL FOLOSESC PENTRU A TRIMITE DATELE LA SLINT
#[derive(Clone)]
struct _DayData {
    datetime: SharedString,
    temp: f32,
    temp_max: f32,
    temp_min: f32,
}

impl From<&ForecastDay> for _DayData {
    fn from(day: &ForecastDay) -> Self {
        _DayData {
            datetime: SharedString::from(day.datetime.clone()),
            temp: day.temp,
            temp_max: day.temp_max,
            temp_min: day.temp_min,
        }
    }
}

#[derive(Clone)]
struct _HourData {
    temp: f32,
    index: String,
}

impl From<&ForecastHour> for _HourData {
    fn from(hour: &ForecastHour) -> Self {
        _HourData {
            temp: hour.temp,
            index: match hour.datetime.split(":").next() {
                Some(result) => result.to_string(),
                None => "N/A".to_string(),
            },
        }
    }
}

struct _FavoriteLocation {
    icon: String,
    name: String,
    temp: f32,
    temp_min: f32,
    temp_max: f32,
}

impl From<&FavoriteWeatherResponse> for _FavoriteLocation {
    fn from(resp: &FavoriteWeatherResponse) -> Self {
        _FavoriteLocation {
            icon: resp.days[0].icon.clone(),
            name: resp._resolved_address.clone(),
            temp: resp.days[0].temp,
            temp_max: resp.days[0].tempmax,
            temp_min: resp.days[0].tempmin,
        }
    }
}

fn capitalize_first(string: String) -> String {
    let mut result = String::new();
    let mut first = true;
    for char in string.chars() {
        if first {
            first = false;
            result.push(char.to_ascii_uppercase());
        } else {
            result.push(char);
            if char == ' ' {
                first = true;
            }
        }
    }

    result
}

fn date_to_day(date: String) -> String {
    let format = "%Y-%m-%d";

    let naive_date = match NaiveDate::parse_from_str(date.as_str(), format) {
        Ok(date) => date,
        Err(e) => {
            eprintln!("Error parsing string to date: {e}");
            return "Invalid Date".to_string();
        }
    };

    let weekday = naive_date.weekday();

    weekday.to_string()
}

fn update_hours(ui_weak: slint::Weak<MainWindow>, day: &ForecastDay) {
    let forecast_hours: Vec<Hour> = day
        .hours
        .iter()
        .map(|hour| Hour {
            index: SharedString::from(match hour.datetime.split(":").next() {
                Some(result) => result.to_string(),
                None => "N/A".to_string(),
            }),
            temp: hour.temp,
        })
        .collect();

    if let Err(e) = ui_weak.upgrade_in_event_loop(move |ui| {
        let hours_model = std::rc::Rc::new(slint::VecModel::from(forecast_hours));
        ui.set_hours(hours_model.into());
    }) {
        eprintln!("Failed to update hours in UI: {:?}", e);
    }
}

async fn get_city_weather_info(location: String) -> Result<ForecastResponse, anyhow::Error> {
    let url = format!(
        "https://weather.visualcrossing.com/VisualCrossingWebServices/rest/services/timeline/{location}/next7days?unitGroup=metric&elements=add%3Aaqieur%2Cadd%3Atzoffset%2Cremove%3AdatetimeEpoch%2Cremove%3Adew%2Cremove%3Afeelslikemax%2Cremove%3Afeelslikemin%2Cremove%3Aprecipcover%2Cremove%3Apreciptype%2Cremove%3Apressure%2Cremove%3Asevererisk%2Cremove%3Asolarenergy%2Cremove%3Asolarradiation%2Cremove%3Astations%2Cremove%3Auvindex%2Cremove%3Awindgust&key={API_KEY}&lang=en&options=minuteinterval_10&contentType=json"
    );
    let response_json = reqwest::get(url).await?.text().await?;
    let response: ForecastResponse = serde_json::from_str(&response_json)?;

    Ok(response)
}

async fn get_fav_city_info_by_coords(
    lat: f64,
    lon: f64,
) -> Result<FavoriteWeatherResponse, anyhow::Error> {
    let url = format!(
        "https://weather.visualcrossing.com/VisualCrossingWebServices/rest/services/timeline/{},{}/today?unitGroup=metric&elements=temp,tempmax,tempmin,icon&key={API_KEY}&contentType=json&include=current",
        lat, lon
    );
    let response_json = reqwest::get(url).await?.text().await?;
    let response: FavoriteWeatherResponse = serde_json::from_str(&response_json)?;
    Ok(response)
}

async fn get_fav_city_info_by_coords_with_retry(
    lat: f64,
    lon: f64,
    max_retries: u32,
) -> Result<FavoriteWeatherResponse, anyhow::Error> {
    let mut retries = 0;
    let mut delay = Duration::from_millis(100);

    loop {
        match get_fav_city_info_by_coords(lat, lon).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                if retries >= max_retries {
                    return Err(e);
                }
                eprintln!("Retry {} for {},{}: {}", retries + 1, lat, lon, e);
                tokio::time::sleep(delay).await;
                retries += 1;
                delay *= 2;
            }
        }
    }
}

fn write_favorites_to_file(favorites: Vec<FavoriteLocation>) {
    let path = Path::new("./favorites.json");
    match serde_json::to_string_pretty(&favorites) {
        Ok(json_output) => {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
            {
                Ok(mut file) => match file.write_all(json_output.as_bytes()) {
                    Ok(_) => {
                        println!("Favorites saved successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to write to favorites.json: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("Error opening favorites.json: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Error serializing favorites.json: {}", e);
        }
    }
}

fn read_favorites_from_file() -> Vec<FavoriteLocation> {
    let path = Path::new("./favorites.json");

    match fs::read_to_string(path) {
        Ok(file_content) => {
            if !file_content.trim().is_empty() {
                match serde_json::from_str(&file_content) {
                    Ok(data) => {
                        println!("Read successfully form favorites.json");
                        data
                    }
                    Err(e) => {
                        eprintln!("Error parsing favorites.json ({e}). Creating new list");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        }
        Err(e) => {
            eprintln!("favorites.json doesn't exist ({e}). Creating.");
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(mut file) => match file.write_all(b"[]") {
                    Ok(_) => {
                        println!("Updated favorites.json with []")
                    }
                    Err(e) => {
                        eprintln!("Error updating favorites.json with []: {e}")
                    }
                },
                Err(e) => {
                    eprintln!("Failed to create favorites.json: {}", e);
                }
            }
            Vec::new()
        }
    }
}

fn add_to_favorites(location: FavoriteLocation) {
    let mut favorites = read_favorites_from_file();

    let already_exists = favorites
        .iter()
        .any(|fav| fav.matches(location.lat, location.lon));

    if already_exists {
        println!("'{}' is already favorited", location.address);
        return;
    }

    favorites.push(location);
    write_favorites_to_file(favorites);
}

fn remove_from_favorites(lat: f64, lon: f64) {
    let mut favorites = read_favorites_from_file();
    favorites.retain(|fav| !fav.matches(lat, lon));
    write_favorites_to_file(favorites);
}

fn is_location_favorited(favorites: &[FavoriteLocation], lat: f64, lon: f64) -> bool {
    favorites.iter().any(|fav| fav.matches(lat, lon))
}

async fn display_weather_info(
    input_location: String,
    ui_weak: slint::Weak<MainWindow>,
) -> Result<ForecastResponse, anyhow::Error> {
    let value = get_city_weather_info(input_location.clone()).await?;
    let today = value
        .days
        .first()
        .ok_or_else(|| anyhow::anyhow!("API returned no forecast days"))?;

    let temp = today.temp;
    let icon = today.icon.clone();
    let address = value.resolved_address.clone();
    let conditions = today.conditions.clone();
    let temp_min = today.temp_min;
    let temp_max = today.temp_max;
    let feels_like = today.feels_like;
    let precip = today.precip;
    let humidity = today.humidity;
    let precip_prob = today.precip_prob;
    let snow = today.snow;
    let snow_depth = today.snow_depth;
    let wind_speed = today.wind_speed;
    let wind_dir = today.wind_dir;
    let aqi_val = today.aqieur.unwrap_or(0.0);

    let forecast_days: Vec<Day> = value
        .days
        .iter()
        .map(|day| Day {
            datetime: SharedString::from(date_to_day(day.datetime.clone())),
            temp: day.temp,
            temp_max: day.temp_max,
            temp_min: day.temp_min,
        })
        .collect();

    match ui_weak.upgrade_in_event_loop(move |ui| {
        ui.set_main_temp(temp);
        ui.set_main_icon(SharedString::from(icon));
        ui.set_location_name(SharedString::from(address));
        ui.set_location_condition(SharedString::from(conditions));

        ui.set_temp_min(temp_min);
        ui.set_temp_max(temp_max);
        ui.set_temp_feels_like(feels_like);

        ui.set_precip_amount(precip);
        ui.set_precip_humidity(humidity);
        ui.set_precip_chance(precip_prob);

        ui.set_snow_amount(snow);
        ui.set_snow_depth_val(snow_depth);

        ui.set_wind_speed_val(wind_speed);
        ui.set_wind_dir_val(wind_dir);
        ui.set_aqi_eur_val(aqi_val);

        let days_model = std::rc::Rc::new(slint::VecModel::from(forecast_days));
        ui.set_days(days_model.into());
    }) {
        Ok(_) => println!("Updated UI successfully!"),
        Err(e) => eprintln!("Failed to update the UI: {e}"),
    }

    update_hours(ui_weak.clone(), today);

    Ok(value)
}

fn update_day_display(ui_weak: slint::Weak<MainWindow>, day: &ForecastDay) {
    let temp = day.temp;
    let icon = day.icon.clone();
    let conditions = day.conditions.clone();
    let temp_min = day.temp_min;
    let temp_max = day.temp_max;
    let feels_like = day.feels_like;
    let precip = day.precip;
    let humidity = day.humidity;
    let precip_prob = day.precip_prob;
    let snow = day.snow;
    let snow_depth = day.snow_depth;
    let wind_speed = day.wind_speed;
    let wind_dir = day.wind_dir;
    let aqi_val = day.aqieur.unwrap_or(0.0);

    if let Err(e) = ui_weak.upgrade_in_event_loop(move |ui| {
        ui.set_main_temp(temp);
        ui.set_main_icon(SharedString::from(icon));
        ui.set_location_condition(SharedString::from(conditions));

        ui.set_temp_min(temp_min);
        ui.set_temp_max(temp_max);
        ui.set_temp_feels_like(feels_like);

        ui.set_precip_amount(precip);
        ui.set_precip_humidity(humidity);
        ui.set_precip_chance(precip_prob);

        ui.set_snow_amount(snow);
        ui.set_snow_depth_val(snow_depth);

        ui.set_wind_speed_val(wind_speed);
        ui.set_wind_dir_val(wind_dir);
        ui.set_aqi_eur_val(aqi_val);
    }) {
        eprintln!("Failed to update day display: {e}")
    }

    update_hours(ui_weak.clone(), day);
}

fn show_error_toast(ui_weak: slint::Weak<MainWindow>, message: String) {
    let ui_weak_clone = ui_weak.clone();

    if let Err(e) = ui_weak.upgrade_in_event_loop(move |ui| {
        ui.set_error_msg(SharedString::from(message));
        ui.set_showing_error(true);
    }) {
        eprintln!("Failed to show error toast: {e}");
        return;
    }

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        if let Err(e) = ui_weak_clone.upgrade_in_event_loop(|ui| {
            ui.set_showing_error(false);
        }) {
            eprintln!("Failed to hide error toast: {e}");
        }
    });
}

slint::include_modules!();
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let main_window = MainWindow::new()?;
    let ui_weak = main_window.as_weak();

    let default_location = "Iasi".to_string();

    let last_forecast: Arc<Mutex<Option<ForecastResponse>>> = Arc::new(Mutex::new(None));
    let last_fav: Arc<Mutex<Vec<FavoriteLocation>>> = Arc::new(Mutex::new(Vec::new()));

    match display_weather_info(default_location.clone(), ui_weak.clone()).await {
        Ok(forecast) => {
            let lat = forecast.latitude;
            let lon = forecast.longitude;

            *last_forecast.lock().await = Some(forecast);
            *last_fav.lock().await = read_favorites_from_file();

            let is_fav = is_location_favorited(&last_fav.lock().await, lat, lon);
            main_window.set_is_favorited(is_fav);

            let last_forecast_for_day_select = last_forecast.clone();
            let last_forecast_for_search = last_forecast.clone();
            let last_forecast_for_fav = last_forecast.clone();

            let last_fav_for_search = last_fav.clone();
            let last_fav_for_fav = last_fav.clone();
            let last_fav_for_fav_panel = last_fav.clone();

            // DAY SELECTED LOGIC
            let ui_weak_for_day = ui_weak.clone();
            main_window.on_day_selected(move |index| {
                let i = index as usize;
                let ui_weak_clone = ui_weak_for_day.clone();
                let forecast_clone = last_forecast_for_day_select.clone();

                tokio::spawn(async move {
                    if let Some(forecast) = forecast_clone.lock().await.as_ref()
                        && i < forecast.days.len()
                    {
                        update_day_display(ui_weak_clone, &forecast.days[i]);
                    }
                });
            });

            // SEARCH LOGIC
            let ui_weak_for_search = ui_weak.clone();

            main_window.on_invoke_api(move |input| {
                let mut location = input.to_string().trim().to_string();

                if location.is_empty() {
                    show_error_toast(
                        ui_weak_for_search.clone(),
                        "Enter a location before searching for one".to_string(),
                    );
                    return;
                }

                location = capitalize_first(location);

                let ui_weak_clone = ui_weak_for_search.clone();
                let forecast_clone = last_forecast_for_search.clone();
                let fav_clone = last_fav_for_search.clone();

                tokio::spawn(async move {
                    match display_weather_info(location.clone(), ui_weak_clone.clone()).await {
                        Ok(forecast) => {
                            let lat = forecast.latitude;
                            let lon = forecast.longitude;
                            *forecast_clone.lock().await = Some(forecast);

                            let favorites = fav_clone.lock().await;
                            let is_fav = is_location_favorited(&favorites, lat, lon);

                            if let Err(e) = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                                ui.set_selected_day_index(0);
                                ui.set_is_favorited(is_fav);
                            }) {
                                eprintln!("Failed to update display after search: {e}");
                            }
                        }
                        Err(e) => {
                            show_error_toast(
                                ui_weak_clone.clone(),
                                "Searched location does not exist!".to_string(),
                            );
                            eprintln!("Error at search: {e}");
                        }
                    }
                });
            });

            // FAVORITE LOGIC
            main_window.on_toggle_fav(move |_location_name, fav| {
                let fav_clone = last_fav_for_fav.clone();
                let forecast_clone = last_forecast_for_fav.clone();

                tokio::spawn(async move {
                    if let Some(forecast) = forecast_clone.lock().await.as_ref() {
                        let lat = forecast.latitude;
                        let lon = forecast.longitude;
                        let address = forecast.resolved_address.clone();

                        if fav {
                            let location = FavoriteLocation::new(address, lat, lon);
                            add_to_favorites(location);
                        } else {
                            remove_from_favorites(lat, lon);
                        }

                        *fav_clone.lock().await = read_favorites_from_file();
                    }
                });
            });

            // FAVORITE PANEL LOGIC
            let ui_weak_for_fav_panel = ui_weak.clone();
            main_window.on_invoke_favorites_api(move || {
                let fav_clone = last_fav_for_fav_panel.clone();
                let ui_weak_clone = ui_weak_for_fav_panel.clone();

                tokio::spawn(async move {
                    let favorites_list = fav_clone.lock().await.clone();

                    let mut tasks = Vec::new();
                    for fav_location in favorites_list {
                        let lat = fav_location.lat;
                        let lon = fav_location.lon;
                        let saved_name = fav_location.address.clone();
                        tasks.push(tokio::spawn(async move {
                            match get_fav_city_info_by_coords_with_retry(lat, lon, 3).await {
                                Ok(data) => Some((data, saved_name)),
                                Err(e) => {
                                    eprintln!("Error getting location ({}, {}): {}", lat, lon, e);
                                    None
                                }
                            }
                        }));
                    }

                    let mut results = Vec::new();
                    for task in tasks {
                        match task.await {
                            Ok(Some(data)) => {
                                results.push(data);
                            }
                            Ok(None) => {
                                eprintln!("Error getting location");
                            }
                            Err(e) => {
                                eprintln!("Task join error: {}", e);
                            }
                        }
                    }

                    let favorite_locations: Vec<FavoritedLocation> = results
                        .iter()
                        .map(|(resp, saved_name)| FavoritedLocation {
                            icon: SharedString::from(resp.days[0].icon.clone()),
                            name: SharedString::from(saved_name.clone()),
                            temp: resp.days[0].temp,
                            tempmin: resp.days[0].tempmin,
                            tempmax: resp.days[0].tempmax,
                        })
                        .collect();

                    if let Err(e) = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                        let favorites_model =
                            std::rc::Rc::new(slint::VecModel::from(favorite_locations));
                        ui.set_favorites(favorites_model.into());
                    }) {
                        eprintln!("Failed to update favorites panel: {e}");
                    }
                });
            });

            if let Err(e) = main_window.run() {
                eprintln!("Failed to run window {e}")
            }
        }
        Err(e) => {
            eprintln!("EROARE: {}", e);
        }
    }

    Ok(())
}

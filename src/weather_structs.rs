use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct ForecastResponse {
    #[serde(rename = "resolvedAddress")]
    pub resolved_address: String,

    pub latitude: f64,
    pub longitude: f64,

    pub days: Vec<ForecastDay>,
}

#[derive(Deserialize, Debug)]
pub struct ForecastDay {
    pub datetime: String,

    #[serde(rename = "tempmax")]
    pub temp_max: f32,
    #[serde(rename = "tempmin")]
    pub temp_min: f32,
    pub temp: f32,
    #[serde(rename = "feelslike")]
    pub feels_like: f32,

    pub humidity: f32,
    pub precip: f32,
    #[serde(rename = "precipprob")]
    pub precip_prob: f32,

    pub snow: f32,
    #[serde(rename = "snowdepth")]
    pub snow_depth: f32,

    #[serde(rename = "windspeed")]
    pub wind_speed: f32,
    #[serde(rename = "winddir")]
    pub wind_dir: f32,
    pub aqieur: Option<f32>,

    pub conditions: String,
    pub icon: String,
    pub hours: Vec<ForecastHour>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ForecastHour {
    pub temp: f32,
    pub datetime: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FavDay {
    #[serde(rename = "tempmax")]
    pub tempmax: f32,
    #[serde(rename = "tempmin")]
    pub tempmin: f32,
    pub temp: f32,
    pub icon: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteWeatherResponse {
    #[serde(rename = "resolvedAddress")]
    pub _resolved_address: String, // am pus underline pt ca imi da warning chiar daca il folosesc atunci cand trimit la slint

    pub days: Vec<FavDay>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FavoriteLocation {
    pub address: String,
    pub lat: f64,
    pub lon: f64,
}

impl FavoriteLocation {
    pub fn new(address: String, lat: f64, lon: f64) -> Self {
        Self { address, lat, lon }
    }

    pub fn matches(&self, lat: f64, lon: f64) -> bool {
        const EPSILON: f64 = 0.01;
        (self.lat - lat).abs() < EPSILON && (self.lon - lon).abs() < EPSILON
    }
}

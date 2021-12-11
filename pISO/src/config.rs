use serde::de::{Deserialize, Deserializer};
use std::time;

fn from_millis<'de, D>(deserializer: D) -> ::std::result::Result<time::Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let val = u64::deserialize(deserializer)?;
    Ok(time::Duration::from_millis(val))
}

#[derive(Clone, Debug, Deserialize)]
pub struct UiConfig {
    pub size_step: f32,
    pub default_size: f32,

    #[serde(deserialize_with = "from_millis")]
    pub min_button_press: time::Duration,

    #[serde(deserialize_with = "from_millis")]
    pub button_long_press: time::Duration,

    pub sort_drives: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserConfig {
    pub name: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DriveConfig {
    pub name: String,
    pub newname: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WifiApConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WifiClientNetworkConfig {
    pub ssid: String,
    pub password: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WifiConfig {
    pub client: Option<Vec<WifiClientNetworkConfig>>,
    pub ap: WifiApConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SystemConfig {
    pub auto_fstrim: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct pinConfig {
    pub up_input: u64,
    pub down_input: u64,
    pub select_input: u64,
    pub dc_pin: u64,
    pub rst_pin: u64, 
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub user: UserConfig,
    pub wifi: WifiConfig,
    pub ui: UiConfig,
    pub drive: Option<Vec<DriveConfig>>,
    pub system: Option<SystemConfig>,
    pub pins: pinConfig
}

#[cfg(test)]
mod tests {
    use toml;
    use super::*;
    #[test]
    fn load_test() {
        let toml_str = r#"
          [ui]
          size_step=5
          default_size=50
          min_button_press=300
          button_long_press=2000
          sort_drives=true

          [system]
          auto_fstrim=true

          [user]
          name="piso"
          password="password"

          [[drive]]
          name="Drive1"
          newname="My Name"

          [[drive]]
          name="Drive2"
          newname="Other Drive"

          [[wifi.client]]
          ssid="home-ap"
          password="faz"

          [[wifi.client]]
          ssid="test"
          password="foobar"

          [pins]
          up_input=21
          down_input=16
          select_input=20
          dc_pin=24
          rst_pin=25

          [wifi.ap]
          ssid="piso"
          password="piso"
        "#;

        let _: Config = toml::from_str(toml_str).unwrap();
    }

    #[test]
    fn load_with_no_wifi_client() {
        let toml_str = r#"
          [ui]
          size_step=4.2
          default_size=50
          min_button_press=300
          button_long_press=2000

          [user]
          name="piso"
          password="password"

          [wifi.ap]
          ssid="piso"
          password="piso"

          [pins]
          up_input=21
          down_input=16
          select_input=20
          dc_pin=24
          rst_pin=25
        "#;

        let _: Config = toml::from_str(toml_str).unwrap();
    }
}

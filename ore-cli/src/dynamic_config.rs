use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct SuggestedBus {
    pub id: usize,
    pub priority_fee: u64,
}

#[derive(Deserialize, Debug)]
pub struct DynamicConfig {
    pub busses: Vec<SuggestedBus>,
}

use crate::Miner;

impl Miner {

  pub async fn get_dynamic_config(&self) -> Option<DynamicConfig> {
    let response = reqwest::get("http://127.0.0.1:8000").await;
    let Ok(data) = response else {
      println!("Failed to get dynamic config");
      return None;
    };

    let Ok(config) = data.json::<DynamicConfig>().await else {
      println!("Failed to parse local config");
      return None
    };

    Some(config)
  }

}
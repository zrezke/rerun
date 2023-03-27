use super::depthai;
use poll_promise::Promise;

const DEPTHAI_API_URL: &str = "http://localhost:8000";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiError {
    pub detail: String,
}

impl Default for ApiError {
    fn default() -> Self {
        Self {
            detail: "ApiError".to_string(),
        }
    }
}

#[derive(Default)]
pub struct Promises {
    pub get_devices: Option<Promise<Result<Vec<depthai::DeviceId>, ApiError>>>,
    select_device: Option<Promise<Result<depthai::Device, ApiError>>>,
}

#[derive(Default)]
pub struct Api {
    pub pending: Promises,
}

struct DevicesResponseBody {}

impl Api {
    pub fn get_devices(&mut self) -> Option<Result<Vec<depthai::DeviceId>, ApiError>> {
        if let Some(promise) = self.pending.get_devices.take() {
            if let Some(response) = promise.ready() {
                self.pending.get_devices = None;
                return Some(response.clone());
            }
        }
        self.pending.get_devices = {
            re_log::info!("Fetching devices from DepthAI API");
            let (sender, promise) = Promise::new();
            let request = ehttp::Request::get(format!("{DEPTHAI_API_URL}/devices"));
            ehttp::fetch(request, move |response| match response {
                Ok(response) => {
                    let body = String::from(response.text().unwrap_or_default());
                    if response.ok {
                        let json: Vec<depthai::DeviceId> =
                            serde_json::from_str(&body).unwrap_or_default();
                        sender.send(Ok(json));
                    } else {
                        let error: ApiError = serde_json::from_str(&body).unwrap_or_default();
                        sender.send(Err(error));
                    }
                }
                Err(_) => {
                    sender.send(Err(ApiError::default()));
                }
            });
            Some(promise)
        };
        return None;
    }

    pub fn select_device(
        &mut self,
        device_id: &depthai::DeviceId,
    ) -> Option<Result<depthai::Device, ApiError>> {
        if self.pending.select_device.is_some() {
            let mut promise_ready = false;
            if let Some(response) = self.pending.select_device.as_mut().unwrap().ready() {
                promise_ready = true;
                return Some(response.clone());
            }
            if promise_ready {
                self.pending.select_device = None;
            }
        }
        let (sender, promise) = Promise::new();
        self.pending.select_device = Some(promise);
        let request =
            ehttp::Request::post(format!("{DEPTHAI_API_URL}/devices/{device_id}"), Vec::new());
        ehttp::fetch(request, move |response| match response {
            Ok(response) => {
                let body = String::from(response.text().unwrap_or_default());
                re_log::info!("Body of set: {:?}", body);
                if response.ok {
                    let device: depthai::Device = serde_json::from_str(&body).unwrap_or_default();
                    sender.send(Ok(device));
                } else {
                    let error: ApiError = serde_json::from_str(&body).unwrap_or_default();
                    sender.send(Err(error));
                }
            }
            Err(_) => {
                sender.send(Err(ApiError::default()));
            }
        });
        None
    }
}

use super::depthai;
use super::ws::{BackWsMessage as WsMessage, WebSocket, WsMessageData, WsMessageType};
use poll_promise::Promise;
use serde::{Deserialize, Serialize};

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
    subscribe: Option<Promise<Result<(), ApiError>>>,
    unsubscribe: Option<Promise<Result<SubscriptionResponse, ApiError>>>,
}

#[derive(Default)]
pub struct Api {
    pub pending: Promises,
    ws: Option<WebSocket>,
}

struct DevicesResponseBody {}

#[derive(serde::Serialize)]
struct SubscriptionBodyRepresentation {
    id: u8, // Made with foxglove in mind should be removed probably
    channelId: u8,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct SubscriptionResponse {
    subscriptions: Vec<u8>,
}

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
            re_log::info!("Promise is some");
            let mut promise_ready = false;
            if let Some(response) = self.pending.select_device.as_ref().unwrap().ready() {
                promise_ready = true;
            }
            if promise_ready {
                return Some(
                    self.pending
                        .select_device
                        .take()
                        .unwrap()
                        .ready()
                        .unwrap()
                        .clone(),
                );
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

    pub fn set_subscriptions(
        &mut self,
        subscriptions: &depthai::Subscriptions,
    ) -> Option<Result<&Vec<u8>, &ApiError>> {
        if self.pending.subscribe.is_some() && self.pending.unsubscribe.is_some() {
            let mut promises_ready = false;
            if self.pending.subscribe.as_ref().unwrap().ready().is_some()
                && self.pending.unsubscribe.as_ref().unwrap().ready().is_some()
            {
                promises_ready = true;
            }
            if promises_ready {
                self.pending.subscribe = None;
                let response = self
                    .pending
                    .unsubscribe
                    .as_ref()
                    .take()
                    .unwrap()
                    .ready()
                    .unwrap()
                    .clone();

                match response {
                    Ok(response) => {
                        re_log::info!("!In api: {:?}", response.subscriptions);
                        return Some(Ok(&response.subscriptions));
                    }
                    Err(e) => {
                        return Some(Err(e));
                    }
                }
            }
            return None;
        }
        let mut subs = Vec::new();
        let mut unsubs = Vec::new();
        if subscriptions.color_image {
            subs.push(SubscriptionBodyRepresentation {
                id: depthai::ChannelId::ColorImage as u8, // Made with foxglove in mind
                channelId: depthai::ChannelId::ColorImage as u8,
            });
        } else {
            unsubs.push(depthai::ChannelId::ColorImage as u8);
        }
        if subscriptions.left_image {
            subs.push(SubscriptionBodyRepresentation {
                id: depthai::ChannelId::LeftImage as u8,
                channelId: depthai::ChannelId::LeftImage as u8,
            });
        } else {
            unsubs.push(depthai::ChannelId::LeftImage as u8);
        }
        if subscriptions.right_image {
            subs.push(SubscriptionBodyRepresentation {
                id: depthai::ChannelId::RightImage as u8,
                channelId: depthai::ChannelId::RightImage as u8,
            });
        } else {
            unsubs.push(depthai::ChannelId::RightImage as u8);
        }
        if subscriptions.depth_image {
            subs.push(SubscriptionBodyRepresentation {
                id: depthai::ChannelId::DepthImage as u8,
                channelId: depthai::ChannelId::DepthImage as u8,
            });
        } else {
            unsubs.push(depthai::ChannelId::DepthImage as u8);
        }
        if subscriptions.point_cloud {
            subs.push(SubscriptionBodyRepresentation {
                id: depthai::ChannelId::PointCloud as u8,
                channelId: depthai::ChannelId::PointCloud as u8,
            });
        } else {
            unsubs.push(depthai::ChannelId::PointCloud as u8);
        }

        let subscribe_body = serde_json::to_string(&subs).unwrap_or_default();
        let (subscribe_sender, subscribe_promise) = Promise::new();
        let subscribe_request = ehttp::Request::post(
            format!("{DEPTHAI_API_URL}/subscribe"),
            subscribe_body.into(),
        );
        ehttp::fetch(subscribe_request, move |response| {
            if let Ok(response) = response {
                let body = String::from(response.text().unwrap_or_default());
                if response.ok {
                    subscribe_sender.send(Ok(()));
                } else {
                    let error: ApiError = serde_json::from_str(&body).unwrap_or_default();
                    subscribe_sender.send(Err(error));
                }
            } else {
                subscribe_sender.send(Err(ApiError::default()))
            }
        });
        self.pending.subscribe = Some(subscribe_promise);

        let (unsubscribe_sender, unsubsribe_promise) = Promise::new();
        let unsubscribe_body = serde_json::to_string(&unsubs).unwrap().into_bytes();
        let unsubscribe_request = ehttp::Request::post(
            format!("{DEPTHAI_API_URL}/unsubscribe"),
            unsubscribe_body.into(),
        );
        ehttp::fetch(unsubscribe_request, move |response| {
            if let Ok(response) = response {
                let body = String::from(response.text().unwrap_or_default());
                re_log::info!("Unsubscribe body: {:?}", body);
                if response.ok {
                    re_log::info!("Response ok, body: {:?}", body);
                    let active_subscriptions: SubscriptionResponse =
                        serde_json::from_str(&body).unwrap_or_default();
                    unsubscribe_sender.send(Ok(active_subscriptions))
                } else {
                    let error: ApiError = serde_json::from_str(&body).unwrap_or_default();
                    unsubscribe_sender.send(Err(error))
                }
            } else {
                unsubscribe_sender.send(Err(ApiError::default()))
            }
        });

        self.pending.unsubscribe = Some(unsubsribe_promise);
        None
    }
}

#[derive(Default)]
pub struct BackendCommChannel {
    ws: WebSocket,
}

#[derive(Serialize, Deserialize)]
struct SubscriptionsData {
    subscriptions: Vec<depthai::ChannelId>,
}

#[derive(Serialize, Deserialize)]
struct SetDeviceData {
    device: depthai::DeviceId,
}

impl BackendCommChannel {
    pub fn set_subscriptions(&mut self, subscriptions: &depthai::Subscriptions) {
        let mut subs = Vec::new();

        if subscriptions.color_image {
            subs.push(depthai::ChannelId::ColorImage);
        }
        if subscriptions.left_image {
            subs.push(depthai::ChannelId::LeftImage);
        }
        if subscriptions.right_image {
            subs.push(depthai::ChannelId::RightImage);
        }
        if subscriptions.depth_image {
            subs.push(depthai::ChannelId::DepthImage);
        }
        if subscriptions.point_cloud {
            subs.push(depthai::ChannelId::PointCloud);
        }
        self.ws.send(
            serde_json::to_string(&WsMessage {
                kind: WsMessageType::Subscriptions,
                data: WsMessageData::Subscriptions(subs),
            })
            .unwrap(),
        );
    }

    pub fn set_pipeline(&mut self, config: &depthai::DeviceConfig) {
        self.ws.send(serde_json::to_string(&config).unwrap());
    }

    pub fn receive(&mut self) -> Option<WsMessage> {
        self.ws.receive()
    }

    pub fn get_devices(&mut self) {
        self.ws.send(
            serde_json::to_string(&WsMessage {
                kind: WsMessageType::Devices,
                data: WsMessageData::Devices(Vec::new()),
            })
            .unwrap(),
        );
    }
    pub fn set_device(&mut self, device_id: depthai::DeviceId) {
        self.ws.send(
            serde_json::to_string(&WsMessage {
                kind: WsMessageType::Device,
                data: WsMessageData::Device(depthai::Device { id: device_id }),
            })
            .unwrap(),
        );
    }
}

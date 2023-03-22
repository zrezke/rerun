use ahash::{HashMap, HashMapExt};
use ehttp;
use poll_promise::Promise;
use std::fmt;

#[derive(serde::Deserialize, serde::Serialize, fmt::Debug, PartialEq, Clone, Copy)]
pub enum ColorCameraResolution {
    THE_1080_P,
    THE_4_K,
}

#[derive(serde::Deserialize, serde::Serialize, fmt::Debug, PartialEq, Clone, Copy)]
pub enum MonoCameraResolution {
    THE_400_P,
}

// fmt::Display is used in UI while fmt::Debug is used with the depthai backend api
impl fmt::Display for ColorCameraResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::THE_1080_P => write!(f, "1080p"),
            Self::THE_4_K => write!(f, "4k"),
        }
    }
}

impl fmt::Display for MonoCameraResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::THE_400_P => write!(f, "400p"),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct ColorCameraConfig {
    pub fps: u8,
    pub resolution: ColorCameraResolution,
}

impl Default for ColorCameraConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            resolution: ColorCameraResolution::THE_1080_P,
        }
    }
}

impl fmt::Debug for ColorCameraConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Color camera config: fps: {}, resolution: {:?}",
            self.fps, self.resolution,
        )
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct MonoCameraConfig {
    pub fps: u8,
    pub resolution: MonoCameraResolution,
}

impl Default for MonoCameraConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            resolution: MonoCameraResolution::THE_400_P,
        }
    }
}

impl fmt::Debug for MonoCameraConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mono camera config: fps: {}, resolution: {:?}",
            self.fps, self.resolution,
        )
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct DeviceConfig {
    pub color_camera: ColorCameraConfig,
    pub left_camera: MonoCameraConfig,
    pub right_camera: MonoCameraConfig,
}

#[derive(fmt::Debug, Clone)]
pub struct PipelineState {
    pub started: bool,
    pub message: String,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct DeviceConfigState {
    pub config: DeviceConfig,

    // Is there a nicer way to handle promises?
    #[serde(skip)]
    pub config_update_promise: Option<Promise<Option<PipelineState>>>,
    #[serde(skip)]
    pub pipeline_state: Option<PipelineState>,
}

impl fmt::Debug for DeviceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Device config: {:?} {:?} {:?}",
            self.color_camera, self.left_camera, self.right_camera,
        )
    }
}

#[derive(serde::Deserialize)]
struct PipelineResponse {
    message: String,
}

impl Default for PipelineResponse {
    fn default() -> Self {
        Self {
            message: "Pipeline not started".to_string(),
        }
    }
}

impl DeviceConfigState {
    pub fn set(&mut self, config: &DeviceConfig) {
        if self.config == *config {
            return;
        }
        self.config = *config;
        self.config_update_promise.get_or_insert_with(|| {
            let (sender, promise) = Promise::new();
            let body = serde_json::to_string(&self.config).unwrap().into_bytes();
            let request = ehttp::Request::post("http://localhost:8000/pipeline", body);
            ehttp::fetch(request, move |response| {
                let response = response.unwrap();
                let body = String::from(response.text().unwrap_or_default());
                let json: PipelineResponse = serde_json::from_str(&body).unwrap_or_default();
                let pipeline_state = PipelineState {
                    started: response.ok,
                    message: json.message,
                };
                sender.send(Some(pipeline_state))
            });
            promise
        });
    }
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub device_config: DeviceConfigState,
    #[serde(skip)] // Want to resubscribe to api when app is reloaded
    pub subscriptions: Subscriptions,
    #[serde(skip)]
    pub subscribe_promise: Option<Promise<Result<(), ()>>>,
    #[serde(skip)]
    pub unsubscribe_promise: Option<Promise<Result<(), ()>>>,
}

#[repr(u8)]
enum ChannelId {
    ColorImage,
    LeftImage,
    RightImage,
    DepthImage,
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq)]
pub struct Subscriptions {
    pub color_image: bool,
    pub left_image: bool,
    pub right_image: bool,
    pub depth_image: bool,
}

impl Default for Subscriptions {
    fn default() -> Self {
        Self {
            color_image: false,
            left_image: false,
            right_image: false,
            depth_image: false,
        }
    }
}

impl State {
    /// Set subscriptions internally and send subscribe / unsubscribe requests to the api
    pub fn set_subscriptions(&mut self, subscriptions: &Subscriptions) {
        if self.subscriptions == *subscriptions {
            return;
        }
        self.subscriptions = *subscriptions;

        #[derive(serde::Serialize)]
        struct SubscriptionBodyRepresentation {
            id: u8,
            channelId: u8,
        };

        let mut subs = Vec::new();
        let mut unsubs = Vec::new();
        if self.subscriptions.color_image {
            subs.push(SubscriptionBodyRepresentation {
                id: ChannelId::ColorImage as u8, // Made with foxglove in mind
                channelId: ChannelId::ColorImage as u8,
            });
        } else {
            unsubs.push(ChannelId::ColorImage as u8);
        }
        if self.subscriptions.left_image {
            subs.push(SubscriptionBodyRepresentation {
                id: ChannelId::LeftImage as u8,
                channelId: ChannelId::LeftImage as u8,
            });
        } else {
            unsubs.push(ChannelId::LeftImage as u8);
        }
        if self.subscriptions.right_image {
            subs.push(SubscriptionBodyRepresentation {
                id: ChannelId::RightImage as u8,
                channelId: ChannelId::RightImage as u8,
            });
        } else {
            unsubs.push(ChannelId::RightImage as u8);
        }
        if self.subscriptions.depth_image {
            subs.push(SubscriptionBodyRepresentation {
                id: ChannelId::DepthImage as u8,
                channelId: ChannelId::DepthImage as u8,
            });
        } else {
            unsubs.push(ChannelId::DepthImage as u8);
        }
        let body = serde_json::to_string(&subs).unwrap().into_bytes();

        let (subscribe_sender, subscribe_promise) = Promise::new();

        let subscribe_request = ehttp::Request::post("http://localhost:8000/subscribe", body);

        ehttp::fetch(subscribe_request, move |response| {
            let response = response.unwrap();
            let body = String::from(response.text().unwrap_or_default());
            let json: PipelineResponse = serde_json::from_str(&body).unwrap_or_default();
            if response.ok {
                subscribe_sender.send(Ok(()))
            } else {
                subscribe_sender.send(Err(()))
            }
        });

        let (unsubscribe_sender, unsubsribe_promise) = Promise::new();
        let body = serde_json::to_string(&unsubs).unwrap().into_bytes();
        let unsubscribe_request = ehttp::Request::post("http://localhost:8000/unsubscribe", body);
        ehttp::fetch(unsubscribe_request, move |response| {
            let response = response.unwrap();
            let body = String::from(response.text().unwrap_or_default());
            let json: PipelineResponse = serde_json::from_str(&body).unwrap_or_default();
            if response.ok {
                unsubscribe_sender.send(Ok(()))
            } else {
                unsubscribe_sender.send(Err(()))
            }
        });
        self.subscribe_promise.insert(subscribe_promise);
        self.unsubscribe_promise.insert(unsubsribe_promise);
    }
}

pub type DeviceId = u32;

use super::api;
use ahash::{HashMap, HashMapExt};
use egui_notify::Toasts;
use ehttp;
use poll_promise::Promise;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};
use std::{fmt, future::Future};

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
pub enum BoardSocket {
    AUTO,
    RGB,
    LEFT,
    RIGHT,
    CENTER,
    CAM_A,
    CAM_B,
    CAM_C,
    CAM_D,
    CAM_E,
    CAM_F,
    CAM_G,
    CAM_H,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct MonoCameraConfig {
    pub fps: u8,
    pub resolution: MonoCameraResolution,
    pub board_socket: BoardSocket,
}

impl Default for MonoCameraConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            resolution: MonoCameraResolution::THE_400_P,
            board_socket: BoardSocket::AUTO,
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub enum DepthProfilePreset {
    HIGH_DENSITY,
    HIGH_ACCURACY,
}

impl Default for DepthProfilePreset {
    fn default() -> Self {
        Self::HIGH_DENSITY
    }
}

impl fmt::Display for DepthProfilePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HIGH_DENSITY => write!(f, "High Density"),
            Self::HIGH_ACCURACY => write!(f, "High Accuracy"),
        }
    }
}

pub enum DepthMedianFilter {
    MEDIAN_OFF,
    KERNEL_3x3,
    KERNEL_5x5,
    KERNEL_7x7,
}

impl Default for DepthMedianFilter {
    fn default() -> Self {
        Self::KERNEL_7x7
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Default)]
pub struct DepthConfig {
    pub default_profile_preset: DepthProfilePreset,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct DeviceConfig {
    pub color_camera: ColorCameraConfig,
    pub left_camera: MonoCameraConfig,
    pub right_camera: MonoCameraConfig,
    pub depth_enabled: bool,
    pub depth: Option<DepthConfig>,
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
        self.config.left_camera.board_socket = BoardSocket::LEFT;
        self.config.right_camera.board_socket = BoardSocket::RIGHT;

        self.config_update_promise.get_or_insert_with(|| {
            let (sender, promise) = Promise::new();
            let body = serde_json::to_string(&self.config).unwrap().into_bytes();
            let request = ehttp::Request::post("http://localhost:8000/pipeline", body);
            ehttp::fetch(request, move |response| {
                let response = response.unwrap(); // TODO(filip): Handle error
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq)]
pub struct Device {
    pub id: DeviceId,
    // Add more fields later
}
impl Default for Device {
    fn default() -> Self {
        Self { id: -1 }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(skip)]
    devices_available: Option<Vec<DeviceId>>,
    #[serde(skip)]
    pub selected_device: Option<Device>,
    pub device_config: DeviceConfigState,

    #[serde(skip)] // Want to resubscribe to api when app is reloaded
    pub subscriptions: Subscriptions,
    #[serde(skip)]
    pub subscribe_promise: Option<Promise<Result<(), ()>>>,
    #[serde(skip)]
    pub unsubscribe_promise: Option<Promise<Result<(), ()>>>,
    #[serde(skip)]
    pub api: api::Api,
    #[serde(skip)]
    poll_instant: Option<Instant>, // No default for Instant
    #[serde(skip)]
    toasts: Toasts,
}

impl Default for State {
    fn default() -> Self {
        Self {
            devices_available: None,
            selected_device: None,
            device_config: DeviceConfigState::default(),
            subscriptions: Subscriptions::default(),
            subscribe_promise: None,
            unsubscribe_promise: None,
            api: api::Api::default(),
            poll_instant: Some(Instant::now()),
            toasts: Toasts::new(),
        }
    }
}

#[repr(u8)]
enum ChannelId {
    ColorImage,
    LeftImage,
    RightImage,
    DepthImage,
    PointCloud,
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Default)]
pub struct Subscriptions {
    pub color_image: bool,
    pub left_image: bool,
    pub right_image: bool,
    pub depth_image: bool,
    pub point_cloud: bool,
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
        if self.subscriptions.point_cloud {
            subs.push(SubscriptionBodyRepresentation {
                id: ChannelId::PointCloud as u8,
                channelId: ChannelId::PointCloud as u8,
            });
        } else {
            unsubs.push(ChannelId::PointCloud as u8);
        }
        let body = serde_json::to_string(&subs).unwrap().into_bytes();

        let (subscribe_sender, subscribe_promise) = Promise::new();

        let subscribe_request = ehttp::Request::post("http://localhost:8000/subscribe", body);

        ehttp::fetch(subscribe_request, move |response| {
            let response = response.unwrap();
            let body = String::from(response.text().unwrap_or_default());
            let json: PipelineResponse = serde_json::from_str(&body).unwrap_or_default();
            if response.ok {
                subscribe_sender.send(Ok(()));
            } else {
                subscribe_sender.send(Err(()));
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

    pub fn get_devices(&mut self) -> Vec<DeviceId> {
        // Return stored available devices or fetch them from the api (they get fetched every 30s via poller)
        if let Some(devices) = self.devices_available.clone() {
            return devices;
        }
        Vec::new()
    }

    pub fn update(&mut self) {
        // TODO: Make this async? only if you are a borrowing master
        if let Some(poll_instant) = self.poll_instant {
            if poll_instant.elapsed().as_secs() < 2 {
                return;
            }
            if let Some(result) = self.api.get_devices() {
                // TODO: Show toast if api error
                match result {
                    Ok(devices) => {
                        if devices.contains(&self.selected_device.unwrap_or_default().id) {
                            self.selected_device = None;
                        }
                        self.devices_available = Some(devices.clone());
                        re_log::info!("Devices: {:?}", devices);
                        if self.selected_device.is_none() {
                            if devices.len() > 0 {
                                self.set_device(*devices.first().unwrap());
                            }
                        }
                    }
                    Err(e) => {
                        re_log::info!("Toast?: {:?}", e.detail);
                        // TODO: Add toasts (have to add toast to state to state or create internal representation and publish in state)
                    }
                }
            }
            self.poll_instant = Some(Instant::now());
        } else {
            self.poll_instant = Some(Instant::now());
        }
    }

    pub fn set_device(&mut self, device_id: DeviceId) {
        if let Some(current_device) = self.selected_device {
            if current_device.id == device_id {
                return;
            }
        }
        re_log::info!("Setting device: {:?}", device_id);
        if let Some(result) = self.api.select_device(&device_id) {
            if let Ok(device) = result {
                re_log::info!("Device: {:?}", device.id);
                self.selected_device = Some(device);
                self.device_config = DeviceConfigState::default();
                self.set_subscriptions(&Subscriptions::default());
            }
        }
    }
}

pub type DeviceId = i64; // i64 because of serialization

use super::api::BackendCommChannel;
use super::ws::{BackWsMessage as WsMessage, WsMessageData, WsMessageType};
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, fmt::Debug)]
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
    pub subscriptions: Option<Subscriptions>, // Shown in ui
    previous_subscriptions: Option<Subscriptions>, // Internal, used to recover previous subs and detect changes
    setting_subscriptions: bool,
    #[serde(skip)]
    pub backend_comms: BackendCommChannel,
    #[serde(skip)]
    poll_instant: Option<Instant>,
    #[serde(skip)]
    toasts: Toasts,
}

impl Default for State {
    fn default() -> Self {
        Self {
            devices_available: None,
            selected_device: None,
            device_config: DeviceConfigState::default(),
            subscriptions: None,
            previous_subscriptions: None,
            setting_subscriptions: false,
            backend_comms: BackendCommChannel::default(),
            poll_instant: Some(Instant::now()), // No default for Instant
            toasts: Toasts::new(),
        }
    }
}

#[repr(u8)]
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, fmt::Debug)]
pub enum ChannelId {
    ColorImage,
    LeftImage,
    RightImage,
    DepthImage,
    PointCloud,
}

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, Default, fmt::Debug)]
pub struct Subscriptions {
    pub color_image: bool,
    pub left_image: bool,
    pub right_image: bool,
    pub depth_image: bool,
    pub point_cloud: bool,
}

impl Subscriptions {
    pub fn from_vec(vec: Vec<ChannelId>) -> Self {
        let mut slf = Self::default();
        for channel in vec {
            match channel {
                ChannelId::ColorImage => slf.color_image = true,
                ChannelId::LeftImage => slf.left_image = true,
                ChannelId::RightImage => slf.right_image = true,
                ChannelId::DepthImage => slf.depth_image = true,
                ChannelId::PointCloud => slf.point_cloud = true,
            }
        }
        slf
    }
}

impl State {
    /// Set subscriptions internally and send subscribe / unsubscribe requests to the api
    // pub fn set_subscriptions(&mut self, subscriptions: &Subscriptions) {
    //     if !self.setting_subscriptions {
    //         if let Some(current) = self.subscriptions {
    //             if current == *subscriptions {
    //                 return;
    //             }
    //         }
    //         self.previous_subscriptions = self.subscriptions;
    //         self.subscriptions = Some(*subscriptions);
    //         self.setting_subscriptions = true;
    //     }
    //     if let Some(result) = self.api.set_subscriptions(subscriptions) {
    //         if let Ok(active_subscriptions) = result {
    //             re_log::info!("Active subscriptions: {:?}", active_subscriptions);
    //             // log contains
    //             re_log::info!(
    //                 "Contains color image: {:?}",
    //                 active_subscriptions.contains(&(ChannelId::ColorImage as u8))
    //             );
    //             let mut new_subscriptions = Subscriptions {
    //                 color_image: active_subscriptions.contains(&(ChannelId::ColorImage as u8)),
    //                 left_image: active_subscriptions.contains(&(ChannelId::LeftImage as u8)),
    //                 right_image: active_subscriptions.contains(&(ChannelId::RightImage as u8)),
    //                 depth_image: active_subscriptions.contains(&(ChannelId::DepthImage as u8)),
    //                 point_cloud: active_subscriptions.contains(&(ChannelId::PointCloud as u8)),
    //             };
    //             self.subscriptions = Some(new_subscriptions);
    //             self.setting_subscriptions = false;
    //         } else {
    //             self.subscriptions = self.previous_subscriptions;
    //             self.setting_subscriptions = false;
    //         }
    //     }
    // }

    pub fn set_subscriptions(&mut self, subscriptions: &Subscriptions) {
        if let Some(current_subscriptions) = self.subscriptions {
            if current_subscriptions == *subscriptions {
                return;
            }
        }
        self.backend_comms.set_subscriptions(subscriptions);
        self.subscriptions = Some(*subscriptions);
    }

    pub fn get_devices(&mut self) -> Vec<DeviceId> {
        // Return stored available devices or fetch them from the api (they get fetched every 30s via poller)
        if let Some(devices) = self.devices_available.clone() {
            return devices;
        }
        Vec::new()
    }

    pub fn update(&mut self) {
        if let Some(ws_message) = self.backend_comms.receive() {
            re_log::info!("Received message: {:?}", ws_message);
            match ws_message.data {
                WsMessageData::Subscriptions(subscriptions) => {
                    re_log::info!("Setting subscriptions");
                    let mut subs = Subscriptions::default();
                    for sub in subscriptions {
                        match sub {
                            ChannelId::ColorImage => subs.color_image = true,
                            ChannelId::LeftImage => subs.left_image = true,
                            ChannelId::RightImage => subs.right_image = true,
                            ChannelId::DepthImage => subs.depth_image = true,
                            ChannelId::PointCloud => subs.point_cloud = true,
                        }
                    }
                    self.subscriptions = Some(subs);
                }
                WsMessageData::Devices(devices) => {
                    re_log::info!("Setting devices...");
                    self.devices_available = Some(devices);
                }
                WsMessageData::Pipeline(config) => {
                    re_log::info!("Todo handle pipeline configs");
                }
                WsMessageData::Device(device) => {
                    re_log::info!("Setting device");
                    self.selected_device = Some(device);
                }
                _ => {}
            }
        }

        if let Some(poll_instant) = self.poll_instant {
            if poll_instant.elapsed().as_secs() < 2 {
                return;
            }
            self.backend_comms.get_devices();
            // if let Some(result) = self.api.get_devices() {
            //     // TODO: Show toast if api error
            //     match result {
            //         Ok(devices) => {
            //             // if devices.contains(&self.selected_device.unwrap_or_default().id) {
            //             //     self.selected_device = None;
            //             // }
            //             self.devices_available = Some(devices.clone());
            //             re_log::info!("Devices: {:?}", devices);
            //             if self.selected_device.is_none() {
            //                 if devices.len() > 0 {
            //                     self.set_device(*devices.first().unwrap());
            //                 }
            //             }
            //         }
            //         Err(e) => {
            //             re_log::info!("Toast?: {:?}", e.detail);
            //             // TODO: Add toasts (have to add toast to state to state or create internal representation and publish in state)
            //         }
            //     }
            // }
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

        self.backend_comms.set_device(device_id);
        // if let Some(result) = self.api.select_device(&device_id) {
        //     if let Ok(device) = result {
        //         re_log::info!("Device: {:?}", device.id);
        //         self.selected_device = Some(device);
        //         self.device_config = DeviceConfigState::default();
        //         // self.api.configure_pipeline(self.device_config.config);
        //         self.set_subscriptions(&Subscriptions::default());
        //     }
        // }
    }
}

pub type DeviceId = i64; // i64 because of serialization

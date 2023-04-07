use itertools::Itertools;
use re_data_store::EntityPropertyMap;
use re_log_types::{EntityPath, EntityPathHash};
use std::collections::{BTreeSet, HashMap};

use crate::ui::SpaceViewId;

use super::super::ui::SpaceView;
use super::api::BackendCommChannel;
use super::ws::{BackWsMessage as WsMessage, WsMessageData, WsMessageType};
use instant::Instant;
use std::fmt;
use std::sync::mpsc::channel;

#[derive(serde::Deserialize, serde::Serialize, fmt::Debug, PartialEq, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum ColorCameraResolution {
    THE_1080_P,
    THE_4_K,
}

#[derive(serde::Deserialize, serde::Serialize, fmt::Debug, PartialEq, Clone, Copy)]
#[allow(non_camel_case_types)]
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
#[allow(non_camel_case_types)]
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
#[allow(non_camel_case_types)]
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, fmt::Debug)]
#[allow(non_camel_case_types)]
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Default, fmt::Debug)]
pub struct DepthConfig {
    // TODO:(filip) add a legit depth config, when sdk is more defined
    pub median: DepthMedianFilter,
    pub pointcloud: PointcloudConfig,
}

impl DepthConfig {
    pub fn default_as_option() -> Option<Self> {
        Some(Self::default())
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, Default, fmt::Debug)]
pub struct PointcloudConfig {
    pub enabled: bool,
}

#[derive(Default, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct DeviceConfig {
    pub color_camera: ColorCameraConfig,
    pub left_camera: MonoCameraConfig,
    pub right_camera: MonoCameraConfig,
    #[serde(default = "bool_true")]
    pub depth_enabled: bool, // Much easier to have an explicit bool for checkbox
    #[serde(default = "DepthConfig::default_as_option")]
    pub depth: Option<DepthConfig>,
    pub ai_model: AiModel,
}

#[inline]
fn bool_true() -> bool {
    true
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct DeviceConfigState {
    pub config: DeviceConfig,
    #[serde(skip)]
    pub update_in_progress: bool,
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

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, PartialEq, fmt::Debug)]
pub enum ErrorAction {
    None,
    FullReset,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, fmt::Debug)]
pub struct Error {
    pub action: ErrorAction,
    pub message: String,
}

impl Default for Error {
    fn default() -> Self {
        Self {
            action: ErrorAction::None,
            message: String::from("Invalid message"),
        }
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

#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq, fmt::Debug)]
pub struct AiModel {
    pub path: String,
    pub display_name: String,
}

impl Default for AiModel {
    fn default() -> Self {
        Self {
            path: String::from(""),
            display_name: String::from("No model selected"),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(skip)]
    devices_available: Option<Vec<DeviceId>>,
    #[serde(skip)]
    pub selected_device: Device,
    pub device_config: DeviceConfigState,

    #[serde(skip, default = "all_subscriptions")]
    // Want to resubscribe to api when app is reloaded
    pub subscriptions: Vec<ChannelId>, // Shown in ui
    #[serde(skip)]
    setting_subscriptions: bool,
    #[serde(skip)]
    pub backend_comms: BackendCommChannel,
    #[serde(skip)]
    poll_instant: Option<Instant>,
    #[serde(default = "default_neural_networks")]
    pub neural_networks: Vec<AiModel>,
}

fn all_subscriptions() -> Vec<ChannelId> {
    vec![
        ChannelId::ColorImage,
        ChannelId::LeftMono,
        ChannelId::RightMono,
        ChannelId::DepthImage,
        ChannelId::PointCloud,
    ]
}

fn default_neural_networks() -> Vec<AiModel> {
    vec![
        AiModel::default(),
        AiModel {
            path: String::from("yolo-v3-tiny-tf"),
            display_name: String::from("Yolo (tiny)"),
        },
        AiModel {
            path: String::from("mobilenet-ssd"),
            display_name: String::from("MobileNet SSD"),
        },
        AiModel {
            path: String::from("face-detection-retail-0004"),
            display_name: String::from("Face Detection"),
        },
        AiModel {
            path: String::from("age-gender-recognition-retail-0013"),
            display_name: String::from("Age gender recognition"),
        },
    ]
}

impl Default for State {
    fn default() -> Self {
        Self {
            devices_available: None,
            selected_device: Device::default(),
            device_config: DeviceConfigState::default(),
            subscriptions: all_subscriptions(),
            setting_subscriptions: false,
            backend_comms: BackendCommChannel::default(),
            poll_instant: Some(Instant::now()), // No default for Instant
            neural_networks: default_neural_networks(),
        }
    }
}

#[repr(u8)]
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, fmt::Debug, Hash)]
pub enum ChannelId {
    ColorImage,
    LeftMono,
    RightMono,
    DepthImage,
    PointCloud,
    PinholeCamera,
}

use lazy_static::lazy_static;
lazy_static! {
    static ref DEPTHAI_ENTITY_HASHES: HashMap<EntityPathHash, ChannelId> = HashMap::from([
        (
            EntityPath::from("world/camera/image/rgb").hash(),
            ChannelId::ColorImage,
        ),
        (
            EntityPath::from("Left mono camera").hash(),
            ChannelId::LeftMono,
        ),
        (
            EntityPath::from("Right mono camera").hash(),
            ChannelId::RightMono,
        ),
        (
            EntityPath::from("right mono camera/depth").hash(),
            ChannelId::DepthImage,
        ),
        (
            EntityPath::from("world/point_cloud").hash(),
            ChannelId::PointCloud,
        ),
    ]);
}

impl State {
    pub fn entities_to_remove(&mut self, entity_path: &BTreeSet<EntityPath>) -> Vec<EntityPath> {
        let mut remove_channels = Vec::<ChannelId>::new();
        if let Some(depth) = self.device_config.config.depth {
            if !depth.pointcloud.enabled {
                remove_channels.push(ChannelId::PointCloud);
            }
        } else {
            remove_channels.push(ChannelId::DepthImage);
        }

        entity_path
            .iter()
            .filter_map(|ep| {
                if let Some(channel) = DEPTHAI_ENTITY_HASHES.get(&ep.hash()) {
                    if remove_channels.contains(channel) {
                        return Some(ep.clone());
                    }
                }
                None
            })
            .collect_vec()
    }

    pub fn set_subscriptions_from_space_views(&mut self, visible_space_views: Vec<&SpaceView>) {
        let mut visibilities = HashMap::<ChannelId, Vec<bool>>::from([
            (ChannelId::ColorImage, Vec::new()),
            (ChannelId::LeftMono, Vec::new()),
            (ChannelId::RightMono, Vec::new()),
            (ChannelId::DepthImage, Vec::new()),
            (ChannelId::PointCloud, Vec::new()),
        ]);

        for space_view in visible_space_views.iter() {
            let mut property_map = space_view.data_blueprint.data_blueprints_projected();
            for entity_path in space_view.data_blueprint.entity_paths().iter() {
                if let Some(channel_id) = DEPTHAI_ENTITY_HASHES.get(&entity_path.hash()) {
                    if let Some(visibility) = visibilities.get_mut(channel_id) {
                        visibility.push(property_map.get(entity_path).visible);
                    }
                }
            }
        }

        let mut possible_subscriptions = Vec::<ChannelId>::from([
            ChannelId::ColorImage,
            ChannelId::LeftMono,
            ChannelId::RightMono,
        ]);

        // Non default subscriptions
        if self.device_config.config.depth.is_some() {
            possible_subscriptions.push(ChannelId::DepthImage);
            if let Some(depth) = self.device_config.config.depth {
                if depth.pointcloud.enabled {
                    possible_subscriptions.push(ChannelId::PointCloud);
                }
            }
        }

        let mut subscriptions = visibilities
            .iter()
            .filter_map(|(channel, vis)| {
                if vis.iter().any(|x| *x) {
                    if possible_subscriptions.contains(channel) {
                        return Some(*channel);
                    }
                }
                None
            })
            .collect_vec();

        // Keep subscriptions that should be visible but have not yet been sent by the backend
        for channel in all_subscriptions() {
            if !subscriptions.contains(&channel)
                && possible_subscriptions.contains(&channel)
                && self.subscriptions.contains(&channel)
            {
                subscriptions.push(channel);
            }
        }

        self.set_subscriptions(&subscriptions);
    }

    pub fn set_subscriptions(&mut self, subscriptions: &Vec<ChannelId>) {
        if self.subscriptions.len() == subscriptions.len()
            && self
                .subscriptions
                .iter()
                .all(|channel_id| subscriptions.contains(channel_id))
        {
            return;
        }
        self.backend_comms.set_subscriptions(subscriptions);
        self.subscriptions = subscriptions.clone();
    }

    pub fn get_devices(&mut self) -> Vec<DeviceId> {
        // Return stored available devices or fetch them from the api (they get fetched every 30s via poller)
        if let Some(devices) = self.devices_available.clone() {
            return devices;
        }
        Vec::new()
    }

    pub fn shutdown(&mut self) {
        self.backend_comms.shutdown();
    }

    pub fn update(&mut self) {
        if let Some(ws_message) = self.backend_comms.receive() {
            re_log::debug!("Received message: {:?}", ws_message);
            match ws_message.data {
                WsMessageData::Subscriptions(subscriptions) => {
                    re_log::debug!("Setting subscriptions");
                    self.subscriptions = subscriptions;
                }
                WsMessageData::Devices(devices) => {
                    re_log::debug!("Setting devices...");
                    self.devices_available = Some(devices);
                }
                WsMessageData::Pipeline(config) => {
                    let mut subs = self.subscriptions.clone();
                    if let Some(depth) = config.depth {
                        subs.push(ChannelId::DepthImage);
                        if depth.pointcloud.enabled {
                            subs.push(ChannelId::PointCloud);
                        }
                    }
                    self.device_config.config = config;
                    self.device_config.config.depth_enabled =
                        self.device_config.config.depth.is_some();
                    self.set_subscriptions(&subs);
                    self.device_config.update_in_progress = false;
                }
                WsMessageData::Device(device) => {
                    re_log::debug!("Setting device");
                    self.selected_device = device;
                    self.backend_comms.set_subscriptions(&self.subscriptions);
                    self.backend_comms.set_pipeline(&self.device_config.config);
                    self.device_config.update_in_progress = true;
                }
                WsMessageData::Error(error) => {
                    re_log::error!("Error: {:?}", error.message);
                    self.device_config.update_in_progress = false;
                    match error.action {
                        ErrorAction::None => (),
                        ErrorAction::FullReset => {
                            self.set_device(-1);
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(poll_instant) = self.poll_instant {
            if poll_instant.elapsed().as_secs() < 2 {
                return;
            }
            if self.selected_device.id == -1 {
                self.backend_comms.get_devices();
            }
            self.poll_instant = Some(Instant::now());
        } else {
            self.poll_instant = Some(Instant::now());
        }
    }

    pub fn set_device(&mut self, device_id: DeviceId) {
        if self.selected_device.id == device_id {
            return;
        }
        re_log::debug!("Setting device: {:?}", device_id);
        self.backend_comms.set_device(device_id);
    }

    pub fn set_device_config(&mut self, config: &mut DeviceConfig) {
        if !self
            .backend_comms
            .ws
            .connected
            .load(std::sync::atomic::Ordering::SeqCst)
            || self.selected_device.id == -1
        {
            return;
        }
        config.left_camera.board_socket = BoardSocket::LEFT;
        config.right_camera.board_socket = BoardSocket::RIGHT;
        self.device_config.config = config.clone();
        self.backend_comms.set_pipeline(&self.device_config.config);
        re_log::info!("Creating pipeline...");
        self.device_config.update_in_progress = true;
    }
}

pub type DeviceId = i64; // i64 because of serialization

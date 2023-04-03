use super::depthai;
use super::ws::{BackWsMessage as WsMessage, WebSocket, WsMessageData, WsMessageType};
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
pub struct BackendCommChannel {
    pub ws: WebSocket,
}

impl BackendCommChannel {
    pub fn shutdown(&mut self) {
        self.ws.shutdown();
    }
    pub fn set_subscriptions(&mut self, subscriptions: &depthai::Subscriptions) {
        let mut subs = Vec::new();

        if subscriptions.color_image {
            subs.push(depthai::ChannelId::ColorImage);
        }
        if subscriptions.left_image {
            subs.push(depthai::ChannelId::LeftMono);
        }
        if subscriptions.right_image {
            subs.push(depthai::ChannelId::RightMono);
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
        self.ws.send(
            serde_json::to_string(&WsMessage {
                kind: WsMessageType::Pipeline,
                data: WsMessageData::Pipeline(config.clone()),
            })
            .unwrap(),
        );
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

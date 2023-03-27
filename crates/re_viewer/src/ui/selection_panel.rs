use egui::NumExt as _;
use itertools::Itertools;
use poll_promise::Promise;
use re_data_store::{
    query_latest_single, ColorMap, ColorMapper, EditableAutoValue, EntityPath, EntityProperties,
};
use re_log_types::{
    component_types::{Tensor, TensorDataMeaning},
    TimeType, Transform,
};

use crate::{
    depthai::depthai,
    ui::{view_spatial::SpatialNavigationMode, Blueprint},
    Item, UiVerbosity, ViewerContext,
};

use super::{data_ui::DataUi, space_view::ViewState};

// ---

struct DeviceConfigurationTabViewer {}

pub type Tab = i32;

impl egui_dock::TabViewer for DeviceConfigurationTabViewer {
    type Tab = Tab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Tab {}", tab));
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        format!("Tab {}", tab).into()
    }
}

/// The "Selection View" side-bar.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub(crate) struct SelectionPanel {}

impl SelectionPanel {
    #[allow(clippy::unused_self)]
    pub fn show_panel(
        &mut self,
        ctx: &mut ViewerContext<'_>,
        ui: &mut egui::Ui,
        blueprint: &mut Blueprint,
    ) {
        let panel = egui::SidePanel::right("selection_view")
            .min_width(120.0)
            .default_width(250.0)
            .resizable(true)
            .frame(egui::Frame {
                fill: ui.style().visuals.panel_fill,
                ..Default::default()
            });

        panel.show_animated_inside(
            ui,
            blueprint.selection_panel_expanded,
            |ui: &mut egui::Ui| {
                egui::TopBottomPanel::top("Device configuration").show_inside(ui, |ui| {
                    ctx.re_ui
                        .large_collapsing_header(ui, "Device configuration", true, |ui| {
                            self.device_configuration_ui(ui, ctx);
                        })
                });
                egui::ScrollArea::both()
                    .auto_shrink([true; 2])
                    .show(ui, |ui| {
                        egui::TopBottomPanel::top("selection_panel_title_bar")
                            .exact_height(re_ui::ReUi::title_bar_height())
                            .frame(egui::Frame {
                                inner_margin: egui::Margin::symmetric(
                                    re_ui::ReUi::view_padding(),
                                    0.0,
                                ),
                                ..Default::default()
                            })
                            .show_inside(ui, |ui| {
                                if let Some(selection) = ctx
                                    .rec_cfg
                                    .selection_state
                                    .selection_ui(ctx.re_ui, ui, blueprint)
                                {
                                    ctx.set_multi_selection(selection.iter().cloned());
                                }
                            });

                        egui::ScrollArea::both()
                            .auto_shrink([true; 2])
                            .show(ui, |ui| {
                                egui::Frame {
                                    inner_margin: egui::Margin::same(re_ui::ReUi::view_padding()),
                                    ..Default::default()
                                }
                                .show(ui, |ui| {
                                    self.contents(ui, ctx, blueprint);
                                });
                            });
                    });
            },
        );
    }

    fn device_configuration_ui(&mut self, ui: &mut egui::Ui, ctx: &mut ViewerContext<'_>) {
        // This block handles resetting the depthai_config_state.config_update_promise once it has finished
        // It sets the pipeline state and clears the promise
        let pipeline_state: Option<depthai::PipelineState> =
            match ctx.depthai_state.device_config.config_update_promise {
                Some(ref mut promise) => match promise.ready() {
                    Some(result) => result.as_ref().and_then(|val| Some(val.clone())),
                    None => None,
                },
                None => None,
            };
        if pipeline_state.is_some() {
            ctx.depthai_state.device_config.pipeline_state =
                Some(pipeline_state.as_ref().unwrap().clone());
            ctx.depthai_state.device_config.config_update_promise =
                None::<Promise<Option<depthai::PipelineState>>>;
        }

        let mut available_devices = ctx.depthai_state.get_devices();
        let mut currently_selected_device = ctx
            .depthai_state
            .selected_device
            .clone()
            .unwrap_or_default();
        let mut combo_device: depthai::DeviceId = currently_selected_device.id;
        if currently_selected_device.id != -1 && available_devices.is_empty() {
            available_devices.push(currently_selected_device.id);
        }

        ui.horizontal(|ui| {
            ui.label("Device: ");
            egui::ComboBox::from_id_source("device")
                .width(70.0)
                .selected_text(if combo_device != -1 {
                    combo_device.to_string()
                } else {
                    "No device selected".to_string()
                })
                .show_ui(ui, |ui| {
                    for device in available_devices {
                        ui.selectable_value(&mut combo_device, device, device.to_string());
                    }
                });
        });
        if combo_device != -1 {
            ctx.depthai_state.set_device(combo_device);
        }

        if ctx
            .depthai_state
            .device_config
            .config_update_promise
            .is_some()
        {
            ui.add(egui::Spinner::new());
            return;
        }

        // re_log::info!("pipeline_state: {:?}", pipeline_state);
        let mut device_config = ctx.depthai_state.device_config.config.clone();
        let mut subscriptions = ctx.depthai_state.subscriptions.clone();

        ui.vertical(|ui| {
            ui.collapsing("Color Camera", |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Resolution: ");
                        egui::ComboBox::from_id_source("color_camera_resolution")
                            .width(70.0)
                            .selected_text(format!("{}", device_config.color_camera.resolution))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut device_config.color_camera.resolution,
                                    depthai::ColorCameraResolution::THE_1080_P,
                                    "1080p",
                                );
                                ui.selectable_value(
                                    &mut device_config.color_camera.resolution,
                                    depthai::ColorCameraResolution::THE_4_K,
                                    "4k",
                                );
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("FPS: ");
                        ui.add(egui::DragValue::new(&mut device_config.color_camera.fps));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut subscriptions.color_image, "Show Color camera");
                    });
                });
            });
            ui.collapsing("Left Mono Camera", |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Resolution: ");
                        egui::ComboBox::from_id_source("left_camera_resolution")
                            .width(70.0)
                            .selected_text(format!("{}", device_config.left_camera.resolution))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut device_config.left_camera.resolution,
                                    depthai::MonoCameraResolution::THE_400_P,
                                    "400p",
                                );
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("FPS: ");
                        ui.add(egui::DragValue::new(&mut device_config.left_camera.fps));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut subscriptions.left_image, "Show Left Mono camera");
                    });
                });
            });
            ui.collapsing("Right Mono Camera", |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Resolution: ");
                        egui::ComboBox::from_id_source("right_camera_resolution")
                            .width(70.0)
                            .selected_text(format!("{}", device_config.right_camera.resolution))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut device_config.right_camera.resolution,
                                    depthai::MonoCameraResolution::THE_400_P,
                                    "400p",
                                );
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("FPS: ");
                        ui.add(egui::DragValue::new(&mut device_config.right_camera.fps));
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut subscriptions.right_image, "Show Right Mono camera");
                    });
                });
            });
            let depth_enabled_label = if device_config.depth_enabled {
                "Disable Depth"
            } else {
                "Enable depth"
            };
            ui.collapsing("Depth", |ui| {
                ui.checkbox(&mut device_config.depth_enabled, depth_enabled_label);
                if !device_config.depth_enabled {
                    return;
                }
                let mut depth = device_config.depth.unwrap_or_default();
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Default profile preset:");
                        egui::ComboBox::from_id_source("depth_default_profile_preset")
                            .width(70.0)
                            .selected_text(format!("{}", depth.default_profile_preset))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut depth.default_profile_preset,
                                    depthai::DepthProfilePreset::HIGH_DENSITY,
                                    "High Density",
                                );
                                ui.selectable_value(
                                    &mut depth.default_profile_preset,
                                    depthai::DepthProfilePreset::HIGH_ACCURACY,
                                    "High Accuracy",
                                );
                            })
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut subscriptions.depth_image, "Show Depth");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut subscriptions.point_cloud, "Show Point Cloud");
                    });
                });
                device_config.depth = Some(depth);
            });
        });

        ctx.depthai_state.set_subscriptions(&subscriptions);
        ctx.depthai_state.device_config.set(&device_config);
    }

    #[allow(clippy::unused_self)]
    fn contents(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &mut ViewerContext<'_>,
        blueprint: &mut Blueprint,
    ) {
        crate::profile_function!();

        let query = ctx.current_query();

        if ctx.selection().is_empty() {
            return;
        }

        let num_selections = ctx.selection().len();
        let selection = ctx.selection().to_vec();
        for (i, selection) in selection.iter().enumerate() {
            ui.push_id(i, |ui| {
                what_is_selected_ui(ui, ctx, blueprint, selection);

                if has_data_section(selection) {
                    ctx.re_ui.large_collapsing_header(ui, "Data", true, |ui| {
                        selection.data_ui(ctx, ui, UiVerbosity::All, &query);
                    });
                }

                ctx.re_ui
                    .large_collapsing_header(ui, "Blueprint", true, |ui| {
                        blueprint_ui(ui, ctx, blueprint, selection);
                    });

                if i + 1 < num_selections {
                    // Add space some space between selections
                    ui.add(egui::Separator::default().spacing(24.0).grow(20.0));
                }
            });
        }
    }
}

fn has_data_section(item: &Item) -> bool {
    match item {
        Item::MsgId(_) | Item::ComponentPath(_) | Item::InstancePath(_, _) => true,
        // Skip data ui since we don't know yet what to show for these.
        Item::SpaceView(_) | Item::DataBlueprintGroup(_, _) => false,
    }
}

/// What is selected? Not the contents, just the short id of it.
pub fn what_is_selected_ui(
    ui: &mut egui::Ui,
    ctx: &mut ViewerContext<'_>,
    blueprint: &mut Blueprint,
    item: &Item,
) {
    match item {
        Item::MsgId(msg_id) => {
            ui.horizontal(|ui| {
                ui.label("Message ID:");
                ctx.msg_id_button(ui, *msg_id);
            });
        }
        Item::ComponentPath(re_log_types::ComponentPath {
            entity_path,
            component_name,
        }) => {
            egui::Grid::new("component_path")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Entity:");
                    ctx.entity_path_button(ui, None, entity_path);
                    ui.end_row();

                    ui.label("Component:");
                    ui.label(component_name.short_name())
                        .on_hover_text(component_name.full_name());
                    ui.end_row();
                });
        }
        Item::SpaceView(space_view_id) => {
            if let Some(space_view) = blueprint.viewport.space_view_mut(space_view_id) {
                ui.horizontal(|ui| {
                    ui.label("Space view:");
                    ui.text_edit_singleline(&mut space_view.display_name);
                });
            }
        }
        Item::InstancePath(space_view_id, instance_path) => {
            egui::Grid::new("space_view_id_entity_path").show(ui, |ui| {
                if instance_path.instance_key.is_splat() {
                    ui.label("Entity:");
                } else {
                    ui.label("Entity instance:");
                }
                ctx.instance_path_button(ui, *space_view_id, instance_path);
                ui.end_row();

                if let Some(space_view_id) = space_view_id {
                    if let Some(space_view) = blueprint.viewport.space_view_mut(space_view_id) {
                        ui.label("in Space View:");
                        ctx.space_view_button(ui, space_view);
                        ui.end_row();
                    }
                }
            });
        }
        Item::DataBlueprintGroup(space_view_id, data_blueprint_group_handle) => {
            if let Some(space_view) = blueprint.viewport.space_view_mut(space_view_id) {
                if let Some(group) = space_view
                    .data_blueprint
                    .group_mut(*data_blueprint_group_handle)
                {
                    egui::Grid::new("data_blueprint_group")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Data Group:");
                            ctx.data_blueprint_group_button_to(
                                ui,
                                group.display_name.clone(),
                                space_view.id,
                                *data_blueprint_group_handle,
                            );
                            ui.end_row();

                            ui.label("in Space View:");
                            ctx.space_view_button_to(
                                ui,
                                space_view.display_name.clone(),
                                space_view.id,
                                space_view.category,
                            );
                            ui.end_row();
                        });
                }
            }
        }
    }
}

impl DataUi for Item {
    fn data_ui(
        &self,
        ctx: &mut ViewerContext<'_>,
        ui: &mut egui::Ui,
        verbosity: UiVerbosity,
        query: &re_arrow_store::LatestAtQuery,
    ) {
        match self {
            Item::SpaceView(_) | Item::DataBlueprintGroup(_, _) => {
                // Shouldn't be reachable since SelectionPanel::contents doesn't show data ui for these.
                // If you add something in here make sure to adjust SelectionPanel::contents accordingly.
                debug_assert!(!has_data_section(self));
            }
            Item::MsgId(msg_id) => {
                msg_id.data_ui(ctx, ui, verbosity, query);
            }
            Item::ComponentPath(component_path) => {
                component_path.data_ui(ctx, ui, verbosity, query);
            }
            Item::InstancePath(_, instance_path) => {
                instance_path.data_ui(ctx, ui, verbosity, query);
            }
        }
    }
}

/// What is the blueprint stuff for this item?
fn blueprint_ui(
    ui: &mut egui::Ui,
    ctx: &mut ViewerContext<'_>,
    blueprint: &mut Blueprint,
    item: &Item,
) {
    match item {
        Item::MsgId(_) => {
            // TODO(andreas): Show space views that contains entities that's part of this message.
            ui.weak("(nothing)");
        }

        Item::ComponentPath(component_path) => {
            list_existing_data_blueprints(ui, ctx, component_path.entity_path(), blueprint);
        }

        Item::SpaceView(space_view_id) => {
            ui.horizontal(|ui| {
                if ui
                    .button("Add/remove entities")
                    .on_hover_text("Manually add or remove entities from the Space View.")
                    .clicked()
                {
                    blueprint
                        .viewport
                        .show_add_remove_entities_window(*space_view_id);
                }

                if ui
                    .button("Clone view")
                    .on_hover_text("Create an exact duplicate of this Space View including all blueprint settings")
                    .clicked()
                {
                    if let Some(space_view) = blueprint.viewport.space_view(space_view_id) {
                        let mut new_space_view = space_view.clone();
                        new_space_view.id = super::SpaceViewId::random();
                        blueprint.viewport.add_space_view(new_space_view);
                        blueprint.viewport.mark_user_interaction();
                    }
                }
            });

            ui.add_space(ui.spacing().item_spacing.y);

            if let Some(space_view) = blueprint.viewport.space_view_mut(space_view_id) {
                space_view.selection_ui(ctx, ui);
            }
        }

        Item::InstancePath(space_view_id, instance_path) => {
            if let Some(space_view) = space_view_id
                .and_then(|space_view_id| blueprint.viewport.space_view_mut(&space_view_id))
            {
                if instance_path.instance_key.is_specific() {
                    ui.horizontal(|ui| {
                        ui.label("Part of");
                        ctx.entity_path_button(ui, *space_view_id, &instance_path.entity_path);
                    });
                    // TODO(emilk): show the values of this specific instance (e.g. point in the point cloud)!
                } else {
                    // splat - the whole entity
                    let data_blueprint = space_view.data_blueprint.data_blueprints_individual();
                    let mut props = data_blueprint.get(&instance_path.entity_path);
                    entity_props_ui(
                        ctx,
                        ui,
                        Some(&instance_path.entity_path),
                        &mut props,
                        &space_view.view_state,
                    );
                    data_blueprint.set(instance_path.entity_path.clone(), props);
                }
            } else {
                list_existing_data_blueprints(ui, ctx, &instance_path.entity_path, blueprint);
            }
        }

        Item::DataBlueprintGroup(space_view_id, data_blueprint_group_handle) => {
            if let Some(space_view) = blueprint.viewport.space_view_mut(space_view_id) {
                if let Some(group) = space_view
                    .data_blueprint
                    .group_mut(*data_blueprint_group_handle)
                {
                    entity_props_ui(
                        ctx,
                        ui,
                        None,
                        &mut group.properties_individual,
                        &space_view.view_state,
                    );
                } else {
                    ctx.selection_state_mut().clear_current();
                }
            }
        }
    }
}

fn list_existing_data_blueprints(
    ui: &mut egui::Ui,
    ctx: &mut ViewerContext<'_>,
    entity_path: &EntityPath,
    blueprint: &Blueprint,
) {
    let space_views_with_path = blueprint
        .viewport
        .space_views_containing_entity_path(entity_path);

    if space_views_with_path.is_empty() {
        ui.weak("(Not shown in any Space View)");
        // TODO(andreas): Offer options for adding?
    } else {
        ui.label("Is shown in:");

        ui.indent("list of data blueprints indent", |ui| {
            for space_view_id in &space_views_with_path {
                if let Some(space_view) = blueprint.viewport.space_view(space_view_id) {
                    ctx.entity_path_button_to(
                        ui,
                        Some(*space_view_id),
                        entity_path,
                        &space_view.display_name,
                    );
                }
            }
        });
    }
}

fn entity_props_ui(
    ctx: &mut ViewerContext<'_>,
    ui: &mut egui::Ui,
    entity_path: Option<&EntityPath>,
    entity_props: &mut EntityProperties,
    view_state: &ViewState,
) {
    ui.checkbox(&mut entity_props.visible, "Visible");
    ui.checkbox(&mut entity_props.interactive, "Interactive")
        .on_hover_text("If disabled, the entity will not react to any mouse interaction");

    egui::Grid::new("entity_properties")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Visible history");
            let visible_history = &mut entity_props.visible_history;
            match ctx.rec_cfg.time_ctrl.timeline().typ() {
                TimeType::Time => {
                    let mut time_sec = visible_history.nanos as f32 / 1e9;
                    let speed = (time_sec * 0.05).at_least(0.01);
                    ui.add(
                        egui::DragValue::new(&mut time_sec)
                            .clamp_range(0.0..=f32::INFINITY)
                            .speed(speed)
                            .suffix("s"),
                    )
                    .on_hover_text("Include this much history of the Entity in the Space View.");
                    visible_history.nanos = (time_sec * 1e9).round() as _;
                }
                TimeType::Sequence => {
                    let speed = (visible_history.sequences as f32 * 0.05).at_least(1.0);
                    ui.add(
                        egui::DragValue::new(&mut visible_history.sequences)
                            .clamp_range(0.0..=f32::INFINITY)
                            .speed(speed),
                    )
                    .on_hover_text("Include this much history of the Entity in the Space View.");
                }
            }
            ui.end_row();

            if view_state.state_spatial.nav_mode == SpatialNavigationMode::ThreeD {
                if let Some(entity_path) = entity_path {
                    pinhole_props_ui(ctx, ui, entity_path, entity_props);
                    depth_props_ui(ctx, ui, entity_path, entity_props);
                }
            }
        });
}

fn colormap_props_ui(ui: &mut egui::Ui, entity_props: &mut EntityProperties) {
    ui.checkbox(&mut entity_props.color_mapping, "Color mapping")
        .on_hover_text("Toggles color mapping");
    ui.end_row();

    if !entity_props.color_mapping {
        return;
    }

    let current = *entity_props.color_mapper.get();

    ui.label("Color map");
    egui::ComboBox::from_id_source("color_mapper")
        .selected_text(current.to_string())
        .show_ui(ui, |ui| {
            ui.style_mut().wrap = Some(false);
            ui.set_min_width(64.0);

            // TODO(cmc): that is not ideal but I don't want to import yet another proc-macro...
            let mut add_label = |proposed| {
                if ui
                    .selectable_label(current == proposed, proposed.to_string())
                    .clicked()
                {
                    entity_props.color_mapper = EditableAutoValue::Auto(proposed);
                }
            };

            add_label(ColorMapper::ColorMap(ColorMap::Grayscale));
            add_label(ColorMapper::ColorMap(ColorMap::Turbo));
            add_label(ColorMapper::ColorMap(ColorMap::Viridis));
            add_label(ColorMapper::ColorMap(ColorMap::Plasma));
            add_label(ColorMapper::ColorMap(ColorMap::Magma));
            add_label(ColorMapper::ColorMap(ColorMap::Inferno));
        });

    ui.end_row();
}

fn pinhole_props_ui(
    ctx: &mut ViewerContext<'_>,
    ui: &mut egui::Ui,
    entity_path: &EntityPath,
    entity_props: &mut EntityProperties,
) {
    let query = ctx.current_query();
    if let Some(re_log_types::Transform::Pinhole(_)) =
        query_latest_single::<Transform>(&ctx.log_db.entity_db, entity_path, &query)
    {
        ui.label("Image plane distance");
        let mut distance = *entity_props.pinhole_image_plane_distance.get();
        let speed = (distance * 0.05).at_least(0.01);
        if ui
            .add(
                egui::DragValue::new(&mut distance)
                    .clamp_range(0.0..=1.0e8)
                    .speed(speed),
            )
            .on_hover_text("Controls how far away the image plane is.")
            .changed()
        {
            entity_props.pinhole_image_plane_distance = EditableAutoValue::UserEdited(distance);
        }
        ui.end_row();
    }
}

fn depth_props_ui(
    ctx: &mut ViewerContext<'_>,
    ui: &mut egui::Ui,
    entity_path: &EntityPath,
    entity_props: &mut EntityProperties,
) {
    let query = ctx.current_query();

    // Find closest pinhole transform, if any.
    let mut pinhole_ent_path = None;
    let mut cur_path = Some(entity_path.clone());
    while let Some(path) = cur_path {
        if let Some(re_log_types::Transform::Pinhole(_)) =
            query_latest_single::<Transform>(&ctx.log_db.entity_db, &path, &query)
        {
            pinhole_ent_path = Some(path);
            break;
        }
        cur_path = path.parent();
    }

    // Early out if there's no pinhole transform upwards in the tree.
    let Some(pinhole_ent_path) = pinhole_ent_path else { return; };

    entity_props.backproject_pinhole_ent_path = Some(pinhole_ent_path.clone());

    let tensor = query_latest_single::<Tensor>(&ctx.log_db.entity_db, entity_path, &query);
    if tensor.as_ref().map(|t| t.meaning) == Some(TensorDataMeaning::Depth) {
        ui.checkbox(&mut entity_props.backproject_depth, "Backproject Depth")
            .on_hover_text(
                "If enabled, the depth texture will be backprojected into a point cloud rather \
                than simply displayed as an image.",
            );
        ui.end_row();

        if entity_props.backproject_depth {
            ui.label("Pinhole");
            ctx.entity_path_button(ui, None, &pinhole_ent_path)
                .on_hover_text(
                    "The entity path of the pinhole transform being used to do the backprojection.",
                );
            ui.end_row();

            ui.label("Backproject scale");
            let mut scale = *entity_props.backproject_scale.get();
            let speed = (scale * 0.05).at_least(0.01);
            if ui
                .add(
                    egui::DragValue::new(&mut scale)
                        .clamp_range(0.0..=1.0e8)
                        .speed(speed),
                )
                .on_hover_text("Scales the backprojected point cloud")
                .changed()
            {
                entity_props.backproject_scale = EditableAutoValue::UserEdited(scale);
            }
            ui.end_row();

            ui.label("Backproject radius scale");
            let mut radius_scale = *entity_props.backproject_radius_scale.get();
            let speed = (radius_scale * 0.001).at_least(0.001);
            if ui
                .add(
                    egui::DragValue::new(&mut radius_scale)
                        .clamp_range(0.0..=1.0e8)
                        .speed(speed),
                )
                .on_hover_text("Scales the radii of the points in the backprojected point cloud")
                .changed()
            {
                entity_props.backproject_radius_scale = EditableAutoValue::UserEdited(radius_scale);
            }
            ui.end_row();

            // TODO(cmc): This should apply to the depth map entity as a whole, but for that we
            // need to get the current hardcoded colormapping out of the image cache first.
            colormap_props_ui(ui, entity_props);
        } else {
            entity_props.backproject_pinhole_ent_path = None;
        }
    }
}

use crate::app::{UiMode, UiState};
use dragonhorde_api_client::api::{Api, ApiClient};
use dragonhorde_api_client::models;
use egui::{Button, OpenUrl, Sense, Vec2};
use egui_infinite_scroll::InfiniteScroll;
use std::sync::{Arc, Mutex};
use egui_flex::{item, Flex};
use crate::views::media::ViewerState;

pub(crate) struct SearchState {
    client: Arc<ApiClient>,
    results: Arc<Mutex<Vec<models::Media>>>,
    infinite_scroll: InfiniteScroll<i64, i64>,
    has_tags: Arc<Mutex<Vec<String>>>,
    not_tags: Arc<Mutex<Vec<String>>>,
    tag_text: String,
    search_changed: bool,
}

impl SearchState {
    pub fn new(client: Arc<ApiClient>) -> Self {
        let results: Arc<Mutex<Vec<models::Media>>> = Arc::new(Mutex::new(Vec::new()));
        let _client = client.clone();
        let _results = results.clone();
        let has_tags: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let not_tags: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let _has_tags = has_tags.clone();
        let _not_tags = not_tags.clone();
        let infinite_scroll = InfiniteScroll::new().end_loader_async(move |cursor| {
            let client = _client.clone();
            let __results = _results.clone();
            let __has_tags = _has_tags.clone();
            let __not_tags = _not_tags.clone();
            async move {
                let has_tags = __has_tags.lock().unwrap().clone();
                let not_tags = __not_tags.lock().unwrap().clone();
                let start = cursor.unwrap_or_else(|| 0);
                dbg!(&start);
                let res = match client.search_api().search_get(Some(has_tags), Some(not_tags), Some(start), Some(20)).await {
                    Ok(mut req) => {
                        let v: Vec<i64> = req.result.iter().map(|i| i.id.unwrap()).collect();
                        __results.lock().unwrap().append(&mut req.result);
                        v
                    }
                    Err(e) => {
                        dbg!(format!("Failed To load Media {:?}", e.to_string()));
                        Vec::new()
                    }
                };
                let len = start + (res.len()) as i64;
                Ok((res, Some(len)))
            }
        });
        Self {
            client,
            results,
            infinite_scroll,
            has_tags,
            not_tags,
            tag_text: "".to_string(),
            search_changed: false,
        }
    }
}

pub(crate) fn search_view(
    ui_state: &mut UiState,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
) -> Option<UiMode> {
    let state = &mut ui_state.search_state;
    let mut ret: Option<UiMode> = None;

    if state.search_changed {
        state.search_changed = false;
        state.infinite_scroll.reset();
    }

    egui::SidePanel::left("my_left_panel")
        .resizable(false)
        .exact_width(250f32)
        .show(ctx, |ui| {
            Flex::horizontal()
                .grow_items(1.0)
                .wrap(true)
                .show(ui, |flex| {
                    let mut has_tags = state.has_tags.lock().unwrap().clone();
                    for value in has_tags.iter() {
                        let response = flex.add(item(), Button::new(value));
                        let local_tags = state.has_tags.clone();
                        response.context_menu(|ui| {
                            if ui.button("Remove").clicked() {
                                let mut _local_tags = local_tags.lock().unwrap();
                                match _local_tags.iter().position(|x| x.eq(value)) {
                                    None => {}
                                    Some(position) => {
                                        _local_tags.remove(position);
                                        state.search_changed = true;
                                    }
                                }
                            }
                        });
                    }
                });
            let response = ui.text_edit_singleline(&mut state.tag_text);
            if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) || ui.input(|i| i.key_pressed(egui::Key::Space)) {
               state.has_tags.lock().unwrap().push(state.tag_text.clone());
                state.tag_text.clear();
                state.search_changed = true;
            }
        });
    //
    // egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {});

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("search")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::splat(16.0);
                let item_spacing = ui.spacing_mut().item_spacing.x;
                state.infinite_scroll.ui_custom_layout(
                    ui,
                    10,
                    |ui, start_idx, item| {
                        let total_width = ui.available_width();

                        let mut count = 1;
                        let mut combined_width = 240f32;

                        while combined_width < total_width - item_spacing * (count - 1) as f32
                            && count < item.len()
                        {
                            count += 1;
                            let item = &item[count - 1];
                            let item_aspect_ratio = 240.0 / 240.0;
                            let item_width = 240.0 * item_aspect_ratio;
                            combined_width += item_width;
                        }

                        let scale =
                            (total_width - item_spacing * (count - 1) as f32) / combined_width;

                        let height = 240.0 * scale;
                        ui.horizontal(|ui| {
                            for (idx, item) in item.iter().enumerate().take(count) {
                                let size = Vec2::new(240.0 * scale, height);
                                if ui
                                    .add_sized(
                                        size,
                                        egui::Image::new(format!(
                                            "http://127.0.0.1:8080/v1/media/{}/thumbnail",
                                            item
                                        ))
                                        .maintain_aspect_ratio(true)
                                        .max_height(240f32)
                                        .max_width(240f32),
                                    )
                                    .interact(Sense::click())
                                    .clicked()
                                {
                                    ui_state.viewer_state = ViewerState::new_with_ctx(state.client.clone(), **item, ctx.clone());
                                    ret =
                                        Some(UiMode::ViewerMode);
                                };
                            }
                        });
                        count
                    },
                );
            });
    });
    ret
}

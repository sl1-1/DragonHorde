use std::default::Default;
use crate::app::{UiMode, UiState};
use dragonhorde_api_client::api::{Api, ApiClient};
use dragonhorde_api_client::models::Media;
use eframe::emath::{Align, Vec2};
use egui::{Button, Modifiers, OpenUrl, Ui};
use egui_flex::{Flex, item};
use egui_material_icons::icons;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tldextract::{TldExtractor, TldOption};
use url::Url;

pub(crate) struct ViewerState {
    id: i64,
    client: Arc<ApiClient>,
    tag_edit: TagEdit,
    media: Arc<Mutex<Media>>,
    image: Arc<Mutex<Option<Vec<u8>>>>,
    zoom: f32,
    website_names: HashMap<String, String>,
    show_controls: bool,
}

impl ViewerState {
    pub fn new(client: Arc<ApiClient>, id: i64) -> Self {
        Self {
            id,
            client,
            tag_edit: Default::default(),
            media: Arc::new(Mutex::new(Default::default())),
            image: Arc::new(Mutex::new(None)),
            zoom: 1.0,
            website_names:  HashMap::new(),
            show_controls: false,
        }
    }
    pub fn new_with_ctx(client: Arc<ApiClient>, id: i64, ctx: egui::Context) -> Self {
        let mut website_names: HashMap<String, String> = HashMap::new();
        website_names.insert("furaffinity.net".to_string(), "Furaffinity".to_string());
        website_names.insert("weasyl.com".to_string(), "Weasyl".to_string());
        website_names.insert("e621.net".to_string(), "e621".to_string());
        website_names.insert("inkbunny.net".to_string(), "InkBunny".to_string());
        website_names.insert("bsky.app".to_string(), "Bsky".to_string());
        website_names.insert("twitter.com".to_string(), "Twitter".to_string());
        website_names.insert("x.com".to_string(), "Twitter".to_string());
        let media = Arc::new(Mutex::new(Default::default()));
        let image = Arc::new(Mutex::new(None));
        load_model(id, client.clone(), media.clone(), ctx.clone());
        load_image(id, client.clone(), image.clone(), ctx.clone());

        Self {
            id,
            client,
            tag_edit: Default::default(),
            media,
            image,
            zoom: 1.0,
            website_names,
            show_controls: false,
        }
    }
}

#[derive(Clone, Debug)]
enum TagType {
    Tag,
    Artist,
    Source,
    Collection,
}

#[derive(Clone, Debug)]
struct TagEntry {
    tag_type: TagType,
    display: String,
    content: String,
    group: Option<String>,
}

impl Default for TagEntry {
    fn default() -> Self {
        Self {
            tag_type: TagType::Tag,
            display: "".to_string(),
            content: "".to_string(),
            group: None,
        }
    }
}

struct TagEdit {
    mode: Option<String>,
    text: String,
    focus: bool,
}

impl Default for TagEdit {
    fn default() -> Self {
        Self {
            mode: None,
            text: String::new(),
            focus: false,
        }
    }
}

fn tag_widget(
    name: String,
    values: Vec<TagEntry>,
    default: TagType,
    media: Arc<Mutex<Media>>,
    state: &mut TagEdit,
    ctx: &egui::Context,
    ui: &mut Ui,
    api_client: Arc<ApiClient>,
) {
    ui.set_width(ui.available_width());
    ui.horizontal(|ui| {
        ui.label(&name);
        ui.with_layout(egui::Layout::right_to_left(Align::default()), |ui| {
            if ui.button(icons::ICON_ADD).clicked() {
                state.mode = Some(name.clone());
            };
        });
    });

    // ui.layout().with_main_justify(false);
    Flex::horizontal()
        .grow_items(1.0)
        .wrap(true)
        .show(ui, |flex| {
            for value in values.into_iter() {
                let response = flex.add(item(), Button::new(value.display.clone()));
                response.context_menu(|ui| {
                    if ui.button("Add To Search").clicked() {
                        // search_fn(
                        //     value.clone(),
                        //     media.clone(),
                        //     ctx.clone(),
                        //     api_client.clone(),
                        // );
                    }

                    if ui.button("Remove").clicked() {
                        delete_data(
                            value.clone(),
                            media.clone(),
                            ctx.clone(),
                            api_client.clone(),
                        );
                    }
                });
                match value.tag_type {
                    TagType::Tag => {}
                    TagType::Artist => {}
                    TagType::Source => {
                        if response.clicked() {
                            ctx.open_url(OpenUrl {
                                url: value.content,
                                new_tab: true,
                            })
                        }
                    }
                    TagType::Collection => {}
                }
            }
        });
    match &state.mode {
        None => {}
        Some(current_state) => {
            if current_state.eq(&name) {
                let response = ui.text_edit_singleline(&mut state.text);
                if !state.focus {
                    response.request_focus();
                    state.focus = true;
                }
                if response.changed() {
                    dbg!(&state.text);
                }
                if !response.has_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let new_tag = TagEntry {
                        tag_type: default,
                        group: Some(name.clone()),
                        content: state.text.clone(),
                        ..Default::default()
                    };
                    add_data(new_tag, media.clone(), ctx.clone(), api_client.clone());

                    state.text.clear();
                    state.mode = None;
                    state.focus = false;
                }
            }
        }
    }
}

fn load_model(id: i64, api_client: Arc<ApiClient>, media: Arc<Mutex<Media>>, ctx: egui::Context) {
    tokio::spawn(async move {
        match api_client.media_api().media_id_get(id).await {
            Ok(req) => {
                let mut m = media.lock().unwrap();
                *m = req
            }
            Err(e) => {
                dbg!(format!("Failed To load Media {:?}", e.to_string()));
            }
        };
        ctx.request_repaint();
    });
}

fn load_image(
    id: i64,
    api_client: Arc<ApiClient>,
    image: Arc<Mutex<Option<Vec<u8>>>>,
    ctx: egui::Context,
) {
    tokio::spawn(async move {
        match api_client.media_api().media_id_file_get(id).await {
            Ok(req) => {
                let mut m = image.lock().unwrap();
                *m = Some(req)
            }
            Err(e) => {
                dbg!(format!("Failed To load image {:?}", e.to_string()));
            }
        };
        ctx.request_repaint();
    });
}

fn delete_value(array: &mut Vec<String>, value: &String) {
    match array.iter().position(|x| x.eq(value)) {
        None => {}
        Some(position) => {
            array.remove(position);
        }
    }
}

fn tag_group_add(tag_groups: &mut HashMap<String, Vec<String>>, tag: &String, group: &String) {
    match tag_groups.get_mut(group) {
        None => {
            tag_groups.insert(group.clone(), vec![tag.clone()]);
        }
        Some(g) => {
            g.push(tag.clone());
        }
    }
}

fn tag_group_delete(tag_groups: &mut HashMap<String, Vec<String>>, tag: &String, group: &String){
    if tag_groups.contains_key(group) {
        delete_value(tag_groups.get_mut(group).unwrap(), &tag);
    }
}

fn add_data(
    tag: TagEntry,
    media: Arc<Mutex<Media>>,
    ctx: egui::Context,
    api_client: Arc<ApiClient>,
) {
    let id;
    {
        id = media.lock().unwrap().id.unwrap();
    }
    dbg!(&tag);
    // Todo: Validate input
    tokio::spawn(async move {
        let patch = match tag.tag_type {
            TagType::Tag => {
                let mut tag_groups = media
                    .lock()
                    .unwrap()
                    .tag_groups
                    .clone()
                    .or(Some(HashMap::new()))
                    .unwrap();
                tag_group_add(&mut tag_groups, &tag.content, &tag.group.clone().unwrap());
                Media{
                    tag_groups: Some(tag_groups),
                    ..Media::default()
                }
            }
            TagType::Artist => {
                let mut new_artists = media
                    .lock()
                    .unwrap()
                    .creators
                    .clone()
                    .or(Some(vec![]))
                    .unwrap();
                new_artists.push(tag.content.clone());
                Media{
                    creators: Some(new_artists),
                    ..Media::default()
                    }
            }
            TagType::Source => {
                let mut new_source = media
                    .lock()
                    .unwrap()
                    .sources
                    .clone()
                    .or(Some(vec![]))
                    .unwrap();
                new_source.push(tag.content.clone());
                Media {
                    sources: Some(new_source),
                    ..Media::default()
                }
            }
            TagType::Collection => {
                let mut new_collections = media
                    .lock()
                    .unwrap()
                    .collections
                    .clone()
                    .or(Some(vec![]))
                    .unwrap();
                new_collections.push(tag.content.clone());
                Media {
                    collections: Some(new_collections),
                    ..Media::default()
                }
            }
        };
        match api_client.media_api().media_id_patch(id, patch).await {
            Ok(_) => {
                let mut m = media.lock().unwrap();
                match tag.tag_type {
                    TagType::Tag => {
                        if let Some(tag_groups) = &mut m.tag_groups {
                            tag_group_add(tag_groups, &tag.content, &tag.group.unwrap());
                        }
                    }
                    TagType::Artist => {
                        if let Some(creators) = &mut m.creators {
                            creators.push(tag.content);
                        }
                    }
                    TagType::Source => {
                        if let Some(sources) = &mut m.sources {
                            sources.push(tag.content);
                        }
                    }
                    TagType::Collection => {
                        if let Some(collection) = &mut m.collections {
                            collection.push(tag.content);
                        }
                    }
                }
            }
            Err(e) => {
                dbg!(format!("Failed To add tag {:?}", e.to_string()));
            }
        };
        ctx.request_repaint();
    });
}

fn delete_data(
    tag: TagEntry,
    media: Arc<Mutex<Media>>,
    ctx: egui::Context,
    api_client: Arc<ApiClient>,
) {
    let id;
    {
        id = media.lock().unwrap().id.unwrap();
    }

    tokio::spawn(async move {
        let patch = match tag.tag_type {
            TagType::Tag => {
                let mut tag_groups = media
                    .lock()
                    .unwrap()
                    .tag_groups
                    .clone()
                    .or(Some(HashMap::new()))
                    .unwrap();
                tag_group_delete(&mut tag_groups, &tag.content, &tag.group.clone().unwrap());
                Media{
                    tag_groups: Some(tag_groups),
                    ..Media::default()
                }
            }
            TagType::Artist => {
                let mut new_artists = media
                    .lock()
                    .unwrap()
                    .creators
                    .clone()
                    .or(Some(vec![]))
                    .unwrap();
                delete_value(&mut new_artists, &tag.content);
                Media{
                    creators: Some(new_artists),
                    ..Media::default()
                }
            }
            TagType::Source => {
                let mut new_source = media
                    .lock()
                    .unwrap()
                    .sources
                    .clone()
                    .or(Some(vec![]))
                    .unwrap();
                delete_value(&mut new_source, &tag.content);
                Media {
                    sources: Some(new_source),
                    ..Media::default()
                }
            }
            TagType::Collection => {
                let mut new_collections = media
                    .lock()
                    .unwrap()
                    .collections
                    .clone()
                    .or(Some(vec![]))
                    .unwrap();
                delete_value(&mut new_collections, &tag.content);
                Media {
                    collections: Some(new_collections),
                    ..Media::default()
                }
            }
        };
        match api_client.media_api().media_id_patch(id, patch).await {
            Ok(_) => {
                let mut m = media.lock().unwrap();
                match tag.tag_type {
                    TagType::Tag => {
                        if let Some(tag_groups) = &mut m.tag_groups {
                            tag_group_delete(tag_groups, &tag.content, &tag.group.unwrap());
                        }
                    }
                    TagType::Artist => {
                        if let Some(creators) = &mut m.creators {
                            delete_value(creators, &tag.content);
                        }
                    }
                    TagType::Source => {
                        if let Some(sources) = &mut m.sources {
                            delete_value(sources, &tag.content);
                        }
                    }
                    TagType::Collection => {
                        if let Some(collection) = &mut m.collections {
                            delete_value(collection, &tag.content);
                        }
                    }
                }
            }
            Err(e) => {
                dbg!(format!("Failed To delete tag {:?}", e.to_string()));
            }
        };
        ctx.request_repaint();
    });
}

fn gen_source_links(sources: Vec<String>, websites: &HashMap<String, String>) -> Vec<TagEntry> {
    let mut links: Vec<TagEntry> = Vec::new();
    for value in sources.iter() {
        match Url::parse(value) {
            Ok(url) => match url.scheme() {
                "file" => {
                    links.push(TagEntry {
                        display: value.clone(),
                        content: value.clone(),
                        tag_type: TagType::Source,
                        ..TagEntry::default()
                    });
                }
                _ => {
                    let domain = TldExtractor::new(TldOption::default())
                        .extract(url.domain().expect("Expected Domain"))
                        .expect("Expected TLD");
                    links.push(TagEntry {
                        tag_type: TagType::Source,
                        display: websites
                            .get(
                                format!("{}.{}", domain.domain.unwrap(), domain.suffix.unwrap())
                                    .as_str(),
                            )
                            .unwrap_or_else(|| value)
                            .clone(),
                        content: value.clone(),
                        ..TagEntry::default()
                    });
                }
            },
            Err(_) => {
                links.push(TagEntry {
                    tag_type: TagType::Source,
                    display: value.clone(),
                    content: value.clone(),
                    ..TagEntry::default()
                });
            }
        };
    }
    links
}

pub(crate) fn render_viewer(
    ui_state: &mut UiState,
    ctx: &egui::Context,
    _frame: &mut eframe::Frame,
) -> Option<UiMode> {
    let mut ret: Option<UiMode> = None;
    let state = &mut ui_state.viewer_state;
    let mut zooming = false;

    if !ctx.wants_keyboard_input() && !ctx.wants_pointer_input() {
        let scroll = ctx.input(|i| i.smooth_scroll_delta);
        let modifiers = ctx.input(|i| i.modifiers);
        if modifiers.contains(Modifiers::SHIFT) {
            zooming = true;
            state.zoom = state.zoom + (scroll.x / 100f32);
            if state.zoom < 0f32 {
                state.zoom = 0f32;
            }
            dbg!(&state.zoom);
        }
    }

    let mut media = state.media.lock().unwrap().clone();

    if media.id.is_none() {
        load_model(
            state.id,
            state.client.clone(),
            state.media.clone(),
            ctx.clone(),
        );
    }
    if !media.id.is_none() && state.image.lock().unwrap().is_none() {
        load_image(
            state.id,
            state.client.clone(),
            state.image.clone(),
            ctx.clone(),
        );
    }

    egui::SidePanel::left("my_left_panel")
        .resizable(false)
        .exact_width(250f32)
        .show(ctx, |ui| {
            Flex::horizontal().grow_items(1.0).show(ui, |flex| {
                if flex.add(item(), egui::Button::new("Back")).clicked() {
                    ret = Some(UiMode::SearchMode);
                }
            });
            egui::ScrollArea::new([false, true])
                .id_salt("metadata")
                .show(ui, |ui| {
                    tag_widget(
                        "Artists".to_string(),
                        media
                            .creators
                            .clone()
                            .unwrap_or_else(|| vec![])
                            .into_iter()
                            .map(|t| TagEntry {
                                tag_type: TagType::Source,
                                display: t.clone(),
                                content: t,
                                ..TagEntry::default()
                            })
                            .collect(),
                        TagType::Artist,
                        state.media.clone(),
                        &mut state.tag_edit,
                        &ctx,
                        ui,
                        state.client.clone(),
                    );
                    ui.separator();
                    let tag_groups = media
                        .tag_groups
                        .clone()
                        .unwrap_or_else(|| HashMap::from([(" ".to_string(), vec![])]));
                    ui.horizontal(|ui| {
                        ui.label("Tags".to_string());
                        ui.with_layout(egui::Layout::right_to_left(Align::default()), |ui| {
                            if ui.button(icons::ICON_LIBRARY_ADD).clicked() {
                                state.tag_edit.mode = Some("TagGroup".to_string());
                            };
                        });
                    });
                    match &state.tag_edit.mode {
                        None => {}
                        Some(mode) => {
                            if mode.eq("TagGroup") {
                                let txt = &mut state.tag_edit.text;
                                let response = ui.text_edit_singleline(txt);
                                if !state.tag_edit.focus {
                                    response.request_focus();
                                    state.tag_edit.focus = true;
                                }
                                if response.changed() {}
                                if response.lost_focus()
                                    || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    if let Some(tag_groups) =
                                        &mut state.media.lock().unwrap().tag_groups
                                    {
                                        match tag_groups.get_mut(txt) {
                                            None => {
                                                tag_groups.insert(txt.clone(), vec![]);
                                            }
                                            Some(_) => {}
                                        }
                                    }
                                    txt.clear();
                                    state.tag_edit.mode = None;
                                    state.tag_edit.focus = false;
                                }
                            }
                        }
                    }

                    for group in tag_groups {
                        tag_widget(
                            group.0.clone(),
                            group
                                .1
                                .into_iter()
                                .map(|t| TagEntry {
                                    tag_type: TagType::Tag,
                                    display: t.clone(),
                                    content: t,
                                    group: Some(group.0.clone()),
                                })
                                .collect(),
                            TagType::Tag,
                            state.media.clone(),
                            &mut state.tag_edit,
                            &ctx,
                            ui,
                            state.client.clone(),
                        );
                    }

                    let collections = media.collections.clone().unwrap_or_else(|| vec![]);
                    ui.separator();
                    tag_widget(
                        "Collections".to_string(),
                        collections
                            .into_iter()
                            .map(|t| TagEntry {
                                tag_type: TagType::Collection,
                                display: t.clone(),
                                content: t,
                                ..TagEntry::default()
                            })
                            .collect(),
                        TagType::Collection,
                        state.media.clone(),
                        &mut state.tag_edit,
                        &ctx,
                        ui,
                        state.client.clone(),
                    );

                    let sources = media.sources.clone().unwrap_or_else(|| vec![]);
                    ui.separator();

                    tag_widget(
                        "Sources".to_string(),
                        gen_source_links(sources, &state.website_names),
                        TagType::Source,
                        state.media.clone(),
                        &mut state.tag_edit,
                        &ctx,
                        ui,
                        state.client.clone(),
                    );
                })
        });

    egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
        egui::Frame::default().show(ui, |ui| {
            egui::ScrollArea::new([false, true])
                .id_salt("description")
                .show(ui, |ui| {
                    ui.label("Description goes here");
                });
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let width = ui.available_width();
        let rect = ui.max_rect();
        dbg!(&width);
        ui.centered_and_justified(|ui| {
            egui::ScrollArea::new([true, true])
                .id_salt("image")
                .show(ui, |ui| {
                    let image = state.image.lock().unwrap();
                    if !image.is_none() {
                        state.show_controls = ui
                            .add(
                                egui::Image::from_bytes(
                                    format!("bytes://{}", &state.id),
                                    image.clone().unwrap(),
                                )
                                // egui::Image::new(format!("http://127.0.0.1:8080/v1/media/{}/file", &state.id))
                                .maintain_aspect_ratio(true)
                                .fit_to_fraction(Vec2 {
                                    x: state.zoom,
                                    y: state.zoom,
                                }),
                            )
                            .hovered();
                    } else {
                        ui.label("Loading");
                    }
                    if state.show_controls {
                        // TODO: Set a timer for this to show for a bit, then autohide until a statechange
                        egui::Window::new("Image Controls")
                            .collapsible(false)
                            .title_bar(false)
                            .fixed_size([100.0, 20.0])
                            .fixed_pos([
                                rect.center_bottom().x - (100.0 / 2.0),
                                rect.center_bottom().y - 20.0,
                            ])
                            .show(ctx, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add(Button::new(icons::ICON_ZOOM_IN));
                                    ui.add(Button::new(icons::ICON_VIEW_REAL_SIZE));
                                    if ui.add(Button::new(icons::ICON_ZOOM_IN)).clicked() {
                                        state.zoom = 1f32;
                                    }
                                    ui.add(Button::new(icons::ICON_ZOOM_OUT));
                                });
                            });
                    }
                });
        });
    });
    ret
}

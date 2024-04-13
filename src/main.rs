use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Deref;
use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::{Datelike, Timelike};
use fast_qr::convert::{image::ImageBuilder, Builder, Shape};
use fast_qr::QRBuilder;
use image::{ColorType, DynamicImage, EncodableLayout, GenericImage, GenericImageView, Rgba};
use image::ImageFormat::Png;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::HtmlInputElement;
use yew::{Callback, function_component, Html, html, Properties, use_state, classes, Event, MouseEvent, TargetCast};

fn hide_image(source: &DynamicImage, hide: &DynamicImage, left: i32, top: i32) -> DynamicImage {
    let mut new_image = DynamicImage::new(source.width(), source.height(), ColorType::Rgba8);
    let hw = hide.width() as i32;
    let hh = hide.height() as i32;
    for w in 0..source.width() {
        for h in 0..source.height() {
            unsafe {
                let sp = source.unsafe_get_pixel(w, h);
                if w as i32 >= left && w as i32 - left < hw && h as i32 >= top && h as i32 - top < hh {
                    let hp = hide.unsafe_get_pixel(w - left as u32, h - top as u32).0[3];
                    if hp <= 200 {
                        const ALPHA: f64 = 150.0;
                        let sp = sp.0;
                        new_image.unsafe_put_pixel(w, h, Rgba([
                            ((sp[0] as f64 - (255.0 - ALPHA)) / ALPHA * 255.0) as u8,
                            ((sp[1] as f64 - (255.0 - ALPHA)) / ALPHA * 255.0) as u8,
                            ((sp[2] as f64 - (255.0 - ALPHA)) / ALPHA * 255.0) as u8,
                            ALPHA as u8,
                        ]));
                    } else {
                        new_image.unsafe_put_pixel(w, h, sp);
                    }
                } else {
                    new_image.unsafe_put_pixel(w, h, sp);
                }
            }
        }
    }
    new_image
}

fn hide_message(source: &DynamicImage, message: &str, left: i32, top: i32, size: i32) -> Result<DynamicImage> {
    let qrcode = QRBuilder::new(message).build()?;
    let hide = ImageBuilder::default()
        .shape(Shape::Square)
        .margin(1)
        .background_color([255, 255, 255, 0])
        .fit_width(size as u32)
        .to_bytes(&qrcode)?;
    let hide = image::load_from_memory(hide.as_slice())?;
    Ok(hide_image(source, &hide, left, top))
}

#[derive(PartialEq, Properties, Debug, Clone)]
struct SquareProps {
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub set_left: Callback<i32>,
    pub set_top: Callback<i32>,
    pub size: i32,
}

fn into_style(style: HashMap<&str, &str>) -> String {
    style.iter()
        .map(|(key, value)| format!("{}: {}", key, value))
        .collect::<Vec<String>>()
        .join("; ")
}

fn alert(msg: &str) {
    gloo::utils::window().alert_with_message(msg).expect_throw("alert error");
    log::error!("{}", msg);
}

#[function_component(Square)]
fn square(props: &SquareProps) -> Html {
    let dragging = use_state(|| false);
    let top = use_state(|| 0);
    let left = use_state(|| 0);
    let delta_x = use_state(|| 0);
    let delta_y = use_state(|| 0);
    let cursor = use_state(|| "auto");
    if props.image_width.is_none() || props.image_height.is_none() {
        return html! { <></> };
    }
    let top_px = format!("{}px", top.deref());
    let left_px = format!("{}px", left.deref());
    let size_px = format!("{}px", props.size);
    let origin_size = props.size.clone();
    let style = HashMap::from([
        ("width", size_px.as_str()),
        ("height", size_px.as_str()),
        ("cursor", cursor.deref()),
        ("top", top_px.as_str()),
        ("left", left_px.as_str()),
    ]);
    let style = into_style(style);
    let onmousemove = {
        let dragging = dragging.clone();
        let top = top.clone();
        let left = left.clone();
        let delta_x = delta_x.clone();
        let delta_y = delta_y.clone();
        let cursor = cursor.clone();
        let image_width = props.image_width.clone().unwrap();
        let image_height = props.image_height.clone().unwrap();
        let set_left = props.set_left.clone();
        let set_top = props.set_top.clone();
        Callback::from(move |e: MouseEvent| {
            if let Some(e) = e.dyn_ref::<MouseEvent>() {
                let mut x = e.offset_x() + *left;
                let mut y = e.offset_y() + *top;
                if x <= 10 || y <= 10 || *left + origin_size - x <= 10 || *top + origin_size - y <= 10 {
                    dragging.set(false);
                    cursor.set("auto");
                } else {
                    cursor.set("grab");
                }
                if *dragging {
                    x -= delta_x.deref();
                    y -= delta_y.deref();
                    x = x.max(0).min(image_width - origin_size);
                    y = y.max(0).min(image_height - origin_size);
                    left.set(x.clone());
                    top.set(y.clone());
                    set_left.emit(x);
                    set_top.emit(y);
                }
            }
        })
    };
    let onmouseup = Some({
        let dragging = dragging.clone();
        let cursor = cursor.clone();
        Callback::from(move |_: MouseEvent| {
            if *cursor == "grab" {
                dragging.set(false);
            }
        })
    });
    let onmousedown = {
        let dragging = dragging.clone();
        let delta_x = delta_x.clone();
        let delta_y = delta_y.clone();
        Callback::from(move |e: MouseEvent| {
            let dx = e.offset_x();
            let dy = e.offset_y();
            dragging.set(true);
            delta_x.set(dx);
            delta_y.set(dy);
        })
    };
    html! {
        <div class="square" style={style} onmousedown={onmousedown}
             onmousemove={onmousemove} onmouseup={onmouseup}/>
    }
}

#[function_component(App)]
fn app() -> Html {
    let image_width = use_state(|| None);
    let image_height = use_state(|| None);
    let left = use_state(|| 0);
    let top = use_state(|| 0);
    let size = use_state(|| 100);
    let content = use_state(|| "".to_string());
    let image_file = use_state(|| vec![]);
    let image_show = use_state(|| "".to_string());
    let dest_image = use_state(|| None);
    let dest_image_show = use_state(|| "".to_string());
    let download_name = use_state(|| "".to_string());
    let set_left = {
        let left = left.clone();
        Callback::from(move |x: i32| {
            left.set(x);
        })
    };
    let set_top = {
        let top = top.clone();
        Callback::from(move |x: i32| {
            top.set(x);
        })
    };
    let onchange_size = {
        let size = size.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<HtmlInputElement>();
            if let Some(input) = input {
                let _ = input.value().parse::<i32>().is_ok_and(|x| {
                    if x < 30 {
                        alert("设置的二维码大小不能太小");
                    } else {
                        size.set(x);
                    }
                    true
                });
            }
        })
    };
    let onchange_content = {
        let content = content.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<HtmlInputElement>();
            if let Some(input) = input {
                content.set(input.value());
            }
        })
    };
    // 读取文件
    let onchange_upload = {
        let size = size.clone();
        let image = image_file.clone();
        let image_show = image_show.clone();
        let image_width = image_width.clone();
        let image_height = image_height.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Some(files) = input.files() {
                if files.length() >= 1 {
                    let file: JsFuture = files.get(0).unwrap().array_buffer().into();
                    spawn_local({
                        let size = size.clone();
                        let image = image.clone();
                        let image_show = image_show.clone();
                        let image_width = image_width.clone();
                        let image_height = image_height.clone();
                        async move {
                            let file = file.await;
                            match file {
                                Ok(file) => {
                                    let file = js_sys::Uint8Array::new(&file).to_vec();
                                    match image::load_from_memory(file.as_bytes()) {
                                        Ok(img) => {
                                            if img.width() < 40 || img.height() < 40 {
                                                alert("图片宽或高太小了");
                                                return;
                                            }
                                            let mut image_png: Vec<u8> = Vec::new();
                                            let result = img.write_to(&mut Cursor::new(&mut image_png), Png);
                                            if let Err(err) = result {
                                                let msg = format!("change image to png error: {}", err);
                                                alert(msg.as_str());
                                                return;
                                            }
                                            image_show.set(format!("data:image/png;base64,{}",
                                                                   BASE64_STANDARD.encode(image_png.as_slice())));
                                            image_width.set(Some(img.width() as i32));
                                            image_height.set(Some(img.height() as i32));
                                            let min_size = img.width().min(img.height()) as i32;
                                            if size.deref() > &min_size {
                                                size.set(min_size);
                                            }
                                        }
                                        Err(err) => {
                                            let msg = format!("load image error: {}", err);
                                            alert(msg.as_str());
                                        }
                                    }
                                    image.set(file);
                                }
                                Err(err) => {
                                    log::error!("upload file with error: {:?}", err);
                                }
                            }
                        }
                    });
                }
            }
        })
    };
    // 生成图片
    let onclick_generate = {
        let image_file = image_file.clone();
        let content = content.clone();
        let left = left.clone();
        let top = top.clone();
        let size = size.clone();
        let dest_image = dest_image.clone();
        let dest_image_show = dest_image_show.clone();
        let download_name = download_name.clone();
        Callback::from(move |_| {
            let source = image::load_from_memory(image_file.deref()).expect("load image error");
            match hide_message(&source, content.deref().as_str(), left.deref().clone(), top.deref().clone(), size.deref().clone()) {
                Ok(dest) => {
                    let mut image_png: Vec<u8> = Vec::new();
                    let result = dest.write_to(&mut Cursor::new(&mut image_png), Png);
                    if let Err(err) = result {
                        let msg = format!("change image to png error: {}", err);
                        alert(msg.as_str());
                        return;
                    }
                    dest_image.set(Some(dest));
                    dest_image_show.set(format!("data:image/png;base64,{}",
                                                BASE64_STANDARD.encode(image_png.as_slice())));
                    let now = chrono::Local::now();
                    let name = format!("{}{:0>2}{:0>2}{:0>2}{:0>2}{:0>2}{:0>3}.png", now.year(), now.month(), now.day(),
                                       now.hour(), now.minute(), now.second(), now.nanosecond() / 1000000);
                    download_name.set(name);
                }
                Err(err) => {
                    let msg = format!("generate image error: {}", err);
                    alert(msg.as_str());
                }
            }
        })
    };

    html! {
        <div class={classes!("col")} style="margin: 10px">
            <div class={classes!("row")}>
                <span>{"项目地址："}</span>
                <a href="https://github.com/ConanYu/qrhide">{"https://github.com/ConanYu/qrhide"}</a>
            </div>
            <div class={classes!("row")} style="width: 500px; margin-top: 10px">
                <div class={classes!("input-group", "flex-nowrap")}>
                    <div class="input-group-prepend">
                        <span class="input-group-text" id="addon-wrapping">{"二维码大小"}</span>
                    </div>
                    <input type="text" class={classes!("form-control")} type="number"
                            value={size.deref().to_string()} onchange={onchange_size}/>
                    <div class="input-group-prepend">
                        <span class="input-group-text" id="addon-wrapping">{"px"}</span>
                    </div>
                </div>
            </div>
            <div class={classes!("row")} style="width: 500px; margin-top: 10px">
                <div class={classes!("input-group", "flex-nowrap")}>
                    <div class="input-group-prepend">
                        <span class="input-group-text" id="addon-wrapping">{"二维码内容"}</span>
                    </div>
                    <input type="text" class={classes!("form-control")} type="text"
                            value={content.deref().clone()} onchange={onchange_content}/>
                </div>
            </div>
            <div class={classes!("row")} style="width: 92px; margin-top: 10px">
                <form>
                    <div class={classes!("custom-file")}>
                        <label class={classes!("upload-label")} for="upload-file">{"图片选择"}</label>
                        <input type="file" class="custom-file-input" id="upload-file" onchange={onchange_upload}/>
                    </div>
                </form>
            </div>
            {
                if image_show.deref().len() > 0 {
                    html! {
                        <div class={classes!("row", "justify-content-start")} style="margin-top: 10px">
                            <div class={classes!("col")} style="flex-grow: 0">
                                <div class={classes!("row")}>
                                    {"二维码位置选择："}
                                </div>
                                <div style="position: relative; margin-top: 10px" class={classes!("row")}>
                                    <Square image_width={image_width.deref()} image_height={image_height.deref()}
                                            set_left={set_left} set_top={set_top} size={size.deref()}/>
                                                <img style="user-select: none;" src={image_show.deref().clone()}/>
                                </div>
                                <div class={classes!("row")} style="margin-top: 10px">
                                    <button class={classes!("btn", "btn-primary")} onclick={onclick_generate}>{"确认"}</button>
                                </div>
                            </div>
                            {
                                if dest_image.deref().is_some() {
                                    html! {
                                        <div class={classes!("col")} style="margin-left: 20px">
                                            <div class={classes!("row")}>
                                                {"生成结果："}
                                            </div>
                                            <div style="position: relative; margin-top: 10px" class={classes!("row")}>
                                                <img style="user-select: none;" src={dest_image_show.deref().clone()}/>
                                            </div>
                                            <div class={classes!("row")} style="margin-top: 10px">
                                                <a class={classes!("btn", "btn-primary")} href={dest_image_show.deref().clone()}
                                                   role="button" download={download_name.deref().clone()}>{"下载"}</a>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! { <></> }
                                }
                            }
                        </div>
                    }
                } else {
                    html! { <></> }
                }
            }
        </div>
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
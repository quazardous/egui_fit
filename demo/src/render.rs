//! Draws an element's widget in its slot, in the form and orientation the
//! engine chose. Every widget is a real egui-pop widget editing the
//! item's values.

use crate::catalog::{Item, Kind};
use egui_gridflow::Orientation;
use egui_pop::core::icon::Icon;
use egui_pop::core::input::Input;
use egui_pop::core::kit::Tone;
use egui_pop::core::progress::Meter;
use egui_pop::core::slider::Slider;
use egui_pop::core::widgets::{combo_box, ToggleChip};
use egui_pop::egui::{self, pos2, vec2, Align2, Color32, FontId, Rect, Stroke, Ui, UiBuilder};
use egui_pop::viewport::curve::CurveEditor;
use egui_pop::viewport::histogram::{Channel, Histogram};
use egui_pop::viewport::wheel::{ColorWheel, WheelValue};

/// The drop-down's choices.
const CHOICES: [&str; 4] = ["Linear", "Log", "Filmic", "Sigmoid"];

/// The wheels' names.
const WHEELS: [&str; 3] = ["Lift", "Gamma", "Gain"];

/// Draws `item` in `rect`, as form `form` (the declared index) turned to
/// `orientation`. `time` drives the indicators.
pub fn show(
    ui: &mut Ui,
    item: &mut Item,
    form: usize,
    orientation: Orientation,
    rect: Rect,
    time: f64,
) {
    let clip = rect.intersect(ui.clip_rect());
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .id_salt(("element", item.key))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(clip);
    let ui = &mut child;
    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
    let vertical = orientation == Orientation::Vertical;
    let label = item.label.clone();
    let v = &mut item.values;
    match item.kind {
        Kind::Slider if vertical => {
            vertical_slider(ui, rect, &mut v.number, form == 0);
        }
        Kind::Slider => match form {
            0 | 1 => {
                let text = if form == 0 {
                    label.clone()
                } else {
                    short(&label)
                };
                in_row(ui, rect, |ui| {
                    Slider::new(&mut v.number, &text, 0.0..=100.0)
                        .unit("%")
                        .default(50.0)
                        .show(ui);
                });
            }
            _ => {
                in_row(ui, rect, |ui| {
                    Input::number(&mut v.number, &label)
                        .precision(0)
                        .width(rect.width())
                        .id_salt(("number", item.key))
                        .show(ui);
                });
            }
        },
        Kind::Toggle => {
            let icon = if v.on { Icon::Visible } else { Icon::Hidden };
            ui.put(
                rect.shrink(1.0),
                ToggleChip::new(&mut v.on, " ").glyph(icon).name(&label),
            )
            .on_hover_text(format!("{label}: {}", if v.on { "on" } else { "off" }));
        }
        Kind::Dropdown => {
            let selected = CHOICES[v.choice % CHOICES.len()];
            let text = if form == 0 {
                selected.to_string()
            } else {
                selected.chars().take(1).collect()
            };
            in_row(ui, rect, |ui| {
                combo_box(("choice", item.key))
                    .selected_text(text)
                    .width(rect.width() - 4.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in CHOICES.iter().enumerate() {
                            ui.selectable_value(&mut v.choice, i, *name);
                        }
                    })
                    .response
                    .on_hover_text(format!("{label}: {selected}"));
            });
        }
        Kind::Field => {
            in_row(ui, rect, |ui| {
                let input = Input::number(&mut v.number, &label)
                    .precision(1)
                    .width(rect.width())
                    .id_salt(("field", item.key));
                let input = if form == 0 { input.unit("EV") } else { input };
                input.show(ui);
            });
        }
        Kind::Knob => {
            let text = if form == 0 {
                label.clone()
            } else {
                short(&label)
            };
            in_row(ui, rect, |ui| {
                Slider::new(&mut v.number, &text, 0.0..=100.0)
                    .integer()
                    .default(50.0)
                    .show(ui);
            });
        }
        Kind::Curve => {
            let side = rect.width().min(rect.height());
            let at = Rect::from_center_size(rect.center(), vec2(side, side));
            ui.scope_builder(UiBuilder::new().max_rect(at), |ui| {
                CurveEditor::new(&mut v.curve, &label)
                    .size(vec2(side, side))
                    .show(ui);
            });
        }
        Kind::Wheels => {
            if form == 2 {
                swatches(ui, rect, &v.wheels, vertical);
            } else {
                wheels(ui, rect, &mut v.wheels, vertical);
            }
        }
        Kind::Meter => {
            let level = 0.5 + 0.45 * (time * 1.7 + item.key as f64).sin();
            if vertical {
                vertical_meter(ui, rect, level as f32);
            } else {
                in_row(ui, rect, |ui| {
                    let meter = Meter::new(level, 0.0, 1.0)
                        .threshold(0.75, Tone::Warning)
                        .threshold(0.9, Tone::Danger)
                        .label(&label)
                        .width(rect.width());
                    let meter = if form == 0 {
                        meter.text(format!("{:.0} %", level * 100.0))
                    } else {
                        meter
                    };
                    ui.add(meter);
                });
            }
        }
        Kind::Histogram => {
            let bins = bins(time + item.key as f64);
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                Histogram::new([Channel::new(
                    "Luminance",
                    &bins,
                    Color32::from_rgb(200, 200, 190),
                )])
                .size(rect.size())
                .label(&label)
                .show(ui);
            });
        }
        Kind::Legend => {
            ui.painter().text(
                rect.left_center() + vec2(4.0, 0.0),
                Align2::LEFT_CENTER,
                &label,
                FontId::proportional(13.0),
                ui.visuals().strong_text_color(),
            );
        }
        Kind::Block => {
            let painter = ui.painter();
            let fill = block_color(item.key);
            painter.rect_filled(rect.shrink(1.0), 4.0, fill);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                &label,
                FontId::proportional(12.0),
                Color32::from_gray(20),
            );
        }
    }
}

/// The first three letters of a label.
fn short(label: &str) -> String {
    label.chars().take(3).collect()
}

/// Runs `add` in a row of the widget's height, centred down `rect`.
fn in_row(ui: &mut Ui, rect: Rect, add: impl FnOnce(&mut Ui)) {
    let h = ui.spacing().interact_size.y.min(rect.height());
    let row = Rect::from_center_size(rect.center(), vec2(rect.width(), h));
    ui.scope_builder(UiBuilder::new().max_rect(row), add);
}

/// egui-pop's slider is horizontal only: the vertical orientation uses
/// egui's own slider, on the same value.
fn vertical_slider(ui: &mut Ui, rect: Rect, value: &mut f64, show_value: bool) {
    let text = if show_value { 24.0 } else { 0.0 };
    ui.spacing_mut().slider_width = (rect.height() - text - 6.0).max(8.0);
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Center)),
        |ui| {
            ui.add(
                egui::Slider::new(value, 0.0..=100.0)
                    .vertical()
                    .show_value(show_value)
                    .integer(),
            );
        },
    );
}

/// The three grading wheels, side by side or stacked.
fn wheels(ui: &mut Ui, rect: Rect, values: &mut [WheelValue; 3], vertical: bool) {
    let gap = 4.0;
    let (along, across) = if vertical {
        (rect.height(), rect.width())
    } else {
        (rect.width(), rect.height())
    };
    let side = ((along - 2.0 * gap) / 3.0).min(across).max(8.0);
    for (i, value) in values.iter_mut().enumerate() {
        let offset = i as f32 * (side + gap);
        let min = if vertical {
            pos2(rect.center().x - side / 2.0, rect.top() + offset)
        } else {
            pos2(rect.left() + offset, rect.center().y - side / 2.0)
        };
        let cell = Rect::from_min_size(min, vec2(side, side));
        ui.scope_builder(UiBuilder::new().max_rect(cell).id_salt(i), |ui| {
            ColorWheel::offset(value, WHEELS[i])
                .size(side)
                .luminance(None)
                .show(ui);
        });
    }
}

/// The simplified wheels: the colour each one pushes towards.
fn swatches(ui: &mut Ui, rect: Rect, values: &[WheelValue; 3], vertical: bool) {
    let painter = ui.painter();
    for (i, v) in values.iter().enumerate() {
        let r = if vertical {
            let h = rect.height() / 3.0;
            Rect::from_min_size(rect.min + vec2(0.0, i as f32 * h), vec2(rect.width(), h))
        } else {
            let w = rect.width() / 3.0;
            Rect::from_min_size(rect.min + vec2(i as f32 * w, 0.0), vec2(w, rect.height()))
        };
        let color = egui::ecolor::Hsva::new(v.hue / 360.0, v.chroma, 0.6, 1.0);
        painter.rect_filled(r.shrink(2.0), 3.0, color);
        painter.text(
            r.center(),
            Align2::CENTER_CENTER,
            WHEELS[i],
            FontId::proportional(10.0),
            Color32::from_gray(20),
        );
    }
}

/// A vertical level bar: egui-pop's meter is horizontal only.
fn vertical_meter(ui: &mut Ui, rect: Rect, level: f32) {
    let painter = ui.painter();
    let track = Rect::from_center_size(
        rect.center(),
        vec2((rect.width() * 0.5).max(4.0), rect.height() - 4.0),
    );
    let fill_color = if level > 0.9 {
        Color32::from_rgb(220, 80, 70)
    } else if level > 0.75 {
        Color32::from_rgb(225, 170, 60)
    } else {
        ui.visuals().selection.bg_fill
    };
    painter.rect_filled(track, 2.0, ui.visuals().extreme_bg_color);
    let top = track.bottom() - track.height() * level.clamp(0.0, 1.0);
    painter.rect_filled(
        Rect::from_min_max(pos2(track.left(), top), track.right_bottom()),
        2.0,
        fill_color,
    );
    painter.rect_stroke(
        track,
        2.0,
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
}

/// A slowly moving two-hump histogram.
fn bins(t: f64) -> Vec<f32> {
    (0..64)
        .map(|i| {
            let x = i as f64 / 63.0;
            let a = 0.3 + 0.05 * (t * 0.5).sin();
            let b = 0.7 + 0.05 * (t * 0.3).cos();
            let g = |c: f64, w: f64| (-(x - c) * (x - c) / (2.0 * w * w)).exp();
            (g(a, 0.08) + 0.6 * g(b, 0.12)) as f32
        })
        .collect()
}

/// A pale colour per key, for plain blocks.
fn block_color(key: u64) -> Color32 {
    let hue = (key.wrapping_mul(97) % 360) as f32 / 360.0;
    egui::ecolor::Hsva::new(hue, 0.35, 0.85, 1.0).into()
}

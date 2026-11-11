//! The demo: a resizable box laid out by egui_fit, a palette and a
//! list of elements grouped into cases, an inspector for the selected one,
//! overlays, a hysteresis trail and the engine's ASCII dump.

use std::collections::{BTreeSet, VecDeque};

use crate::catalog::{s, Form, Item, Kind, FORM_NAMES};
use crate::render;
use egui_fit::{
    layout_grouped, AspectRatio, Axis, Cell, Container, Group, Grow, Layout, Orientation,
    Placement, Reason, Size, Status, TWELFTHS_PER_UNIT,
};
use egui_pop::egui::{
    self, pos2, vec2, Align2, Color32, CursorIcon, FontId, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui,
};

/// How many trail samples are kept.
const TRAIL: usize = 240;

/// The sizes the inspector offers, in twelfths.
const SIZES: [u32; 14] = [3, 4, 6, 8, 9, 12, 24, 36, 48, 60, 72, 96, 120, 144];

/// The density, which sets the unit in points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Density {
    Comfortable,
    Compact,
    Touch,
}

impl Density {
    const ALL: [Density; 3] = [Density::Comfortable, Density::Compact, Density::Touch];

    fn name(self) -> &'static str {
        match self {
            Density::Comfortable => "Comfortable",
            Density::Compact => "Compact",
            Density::Touch => "Touch",
        }
    }

    /// The base unit in points: a third of it is one row of controls.
    fn unit(self) -> f32 {
        match self {
            Density::Comfortable => 120.0,
            Density::Compact => 96.0,
            Density::Touch => 144.0,
        }
    }
}

/// The overlays drawn over the box.
#[derive(Clone, Copy, Debug)]
struct Overlays {
    grid: bool,
    status: bool,
    order: bool,
    groups: bool,
}

/// One point of the hysteresis trail.
#[derive(Clone, Copy, Debug)]
struct Sample {
    /// The box width, in twelfths.
    natural: f32,
    /// The grid width the engine kept, in twelfths.
    columns: u32,
    /// Whether a form or an orientation changed at this sample.
    flipped: bool,
}

/// The demo's state.
pub struct Demo {
    items: Vec<Item>,
    next_key: u64,
    selected: Option<u64>,
    /// The elements ticked in the list, to group into a case.
    marked: BTreeSet<u64>,
    form_tab: usize,
    box_width: f32,
    bounded: bool,
    box_height: f32,
    density: Density,
    themed: Option<Density>,
    hysteresis: bool,
    wobble: bool,
    overlays: Overlays,
    last: Option<Layout>,
    last_width: f32,
    trail: VecDeque<Sample>,
    choices: Vec<(u64, usize, Orientation)>,
    flips: u32,
    grid_changes: u32,
    preset: &'static str,
}

impl Default for Demo {
    fn default() -> Self {
        let mut demo = Demo {
            items: Vec::new(),
            next_key: 1,
            selected: None,
            marked: BTreeSet::new(),
            form_tab: 0,
            box_width: 480.0,
            bounded: false,
            box_height: 160.0,
            density: Density::Comfortable,
            themed: None,
            hysteresis: true,
            wobble: false,
            overlays: Overlays {
                grid: true,
                status: true,
                order: false,
                groups: true,
            },
            last: None,
            last_width: 0.0,
            trail: VecDeque::new(),
            choices: Vec::new(),
            flips: 0,
            grid_changes: 0,
            preset: "",
        };
        demo.preset(PRESETS[0]);
        demo
    }
}

/// A preset: its name, its box width in units, its bounded height in
/// units (if any), and its elements.
type Preset = (
    &'static str,
    f32,
    Option<f32>,
    fn(&mut dyn FnMut(Kind) -> Item) -> Vec<Item>,
);

const PRESETS: [Preset; 5] = [
    ("Darkroom card", 4.0, None, darkroom),
    ("Two cases", 4.0, None, cases),
    ("Toolbar strip", 6.0, Some(1.0 / 3.0), toolbar),
    ("Wide dock", 9.0, None, dock),
    ("Gap filling", 3.0, None, gaps),
];

fn darkroom(new: &mut dyn FnMut(Kind) -> Item) -> Vec<Item> {
    vec![
        new(Kind::Toggle).labelled("Enabled"),
        new(Kind::Dropdown).labelled("Mode"),
        new(Kind::Slider).labelled("Exposure"),
        new(Kind::Slider).labelled("Contrast").with_number(40.0),
        new(Kind::Slider).labelled("Highlights").with_number(70.0),
        new(Kind::Slider).labelled("Shadows").with_number(25.0),
        new(Kind::Toggle).labelled("Preserve colours"),
        new(Kind::Curve).labelled("Tone curve"),
        new(Kind::Meter).labelled("Level"),
    ]
}

/// Two cases, each a legend over its controls, among loose controls.
fn cases(new: &mut dyn FnMut(Kind) -> Item) -> Vec<Item> {
    let mut out = vec![
        new(Kind::Toggle).labelled("Enabled"),
        new(Kind::Dropdown).labelled("Mode"),
    ];
    let tone = new(Kind::Legend).labelled("Tone");
    let t = tone.key;
    out.extend([
        tone.in_case(t),
        new(Kind::Slider).labelled("Exposure").in_case(t),
        new(Kind::Toggle).labelled("Auto").in_case(t),
        new(Kind::Slider)
            .labelled("Contrast")
            .with_number(40.0)
            .in_case(t),
        new(Kind::Dropdown).labelled("Curve").in_case(t),
    ]);
    out.push(new(Kind::Knob).labelled("Mix"));
    let colour = new(Kind::Legend).labelled("Colour");
    let c = colour.key;
    out.extend([
        colour.in_case(c),
        new(Kind::Wheels).labelled("Grading").in_case(c),
        new(Kind::Slider)
            .labelled("Saturation")
            .with_number(60.0)
            .in_case(c),
    ]);
    out.extend([
        new(Kind::Meter).labelled("Level"),
        new(Kind::Toggle).labelled("Preview"),
    ]);
    out
}

fn toolbar(new: &mut dyn FnMut(Kind) -> Item) -> Vec<Item> {
    vec![
        new(Kind::Toggle).labelled("Snap"),
        new(Kind::Toggle).labelled("Grid"),
        new(Kind::Toggle).labelled("Guides"),
        new(Kind::Dropdown).labelled("Tool"),
        new(Kind::Field).labelled("Zoom"),
        new(Kind::Knob).labelled("Opacity"),
        new(Kind::Slider).labelled("Brush"),
        new(Kind::Dropdown).labelled("Blend"),
        new(Kind::Knob).labelled("Flow"),
        new(Kind::Meter).labelled("Memory"),
    ]
}

fn dock(new: &mut dyn FnMut(Kind) -> Item) -> Vec<Item> {
    vec![
        new(Kind::Histogram).labelled("Histogram"),
        new(Kind::Wheels).labelled("Grading"),
        new(Kind::Curve).labelled("RGB curve"),
        new(Kind::Slider).labelled("Saturation"),
        new(Kind::Slider).labelled("Vibrance").with_number(30.0),
        new(Kind::Field).labelled("White point"),
        new(Kind::Dropdown).labelled("Space"),
        new(Kind::Meter).labelled("Clipping"),
    ]
}

fn gaps(new: &mut dyn FnMut(Kind) -> Item) -> Vec<Item> {
    use Size as S;
    let block = |new: &mut dyn FnMut(Kind) -> Item, label: &str, w: Size, h: Size| {
        new(Kind::Block).labelled(label).sized(s(w, h))
    };
    vec![
        block(new, "2 x 1", S::units(2), S::ONE),
        block(new, "1/3", S::THIRD, S::HALF),
        block(new, "2/3", S::TWO_THIRDS, S::HALF),
        block(new, "1 x 1/3", S::ONE, S::THIRD),
        block(new, "1/2", S::HALF, S::HALF),
        block(new, "1/2", S::HALF, S::HALF),
        block(new, "1/3", S::THIRD, S::THIRD),
        block(new, "1/3", S::THIRD, S::THIRD),
        block(new, "1/3", S::THIRD, S::THIRD),
        block(new, "1 x 1/2", S::ONE, S::HALF),
        block(new, "2/3", S::TWO_THIRDS, S::THIRD),
        block(new, "1/3", S::THIRD, S::THIRD),
        block(new, "1/2", S::HALF, S::THIRD),
    ]
}

impl Demo {
    fn new_item(&mut self, kind: Kind) -> Item {
        let key = self.next_key;
        self.next_key += 1;
        Item::new(key, kind)
    }

    fn preset(&mut self, (name, units, height, build): Preset) {
        let mut next = self.next_key;
        let mut new = |kind: Kind| {
            let item = Item::new(next, kind);
            next += 1;
            item
        };
        self.items = build(&mut new);
        self.next_key = next;
        let unit = self.density.unit();
        self.box_width = units * unit;
        self.bounded = height.is_some();
        if let Some(h) = height {
            self.box_height = h * unit;
        }
        self.selected = self.items.first().map(|i| i.key);
        self.marked.clear();
        self.form_tab = 0;
        self.preset = name;
        self.reset_trail();
    }

    fn reset_trail(&mut self) {
        self.last = None;
        self.trail.clear();
        self.choices.clear();
        self.flips = 0;
        self.grid_changes = 0;
    }

    fn index_of(&self, key: u64) -> Option<usize> {
        self.items.iter().position(|i| i.key == key)
    }

    /// The engine's groups: each run of items in the same case, with its
    /// legend as the header when it comes first.
    fn groups(&self) -> Vec<Group> {
        let mut groups: Vec<Group> = Vec::new();
        let mut start = 0;
        for i in 1..=self.items.len() {
            let case = self.items[start].case;
            if i < self.items.len() && self.items[i].case == case {
                continue;
            }
            if let Some(key) = case {
                let mut g = Group::new(key, start..i);
                if self.items[start].kind == Kind::Legend {
                    g = g.with_header();
                }
                groups.push(g);
            }
            start = i;
        }
        groups
    }

    /// Groups the marked items into a new case under a legend, at the
    /// place of the first of them.
    fn group_marked(&mut self) {
        let Some(at) = self.items.iter().position(|i| self.marked.contains(&i.key)) else {
            return;
        };
        let legend = self.new_item(Kind::Legend);
        let case = legend.key;
        let cases = self.items.iter().filter(|i| i.kind == Kind::Legend).count();
        let mut members = vec![legend.labelled(&format!("Case {}", cases + 1))];
        let mut rest = Vec::new();
        for item in self.items.drain(..) {
            if self.marked.contains(&item.key) && item.kind != Kind::Legend {
                members.push(item);
            } else {
                rest.push(item);
            }
        }
        for m in &mut members {
            m.case = Some(case);
        }
        let at = at.min(rest.len());
        rest.splice(at..at, members);
        self.items = rest;
        self.drop_empty_cases();
        self.marked.clear();
        self.selected = Some(case);
    }

    /// Takes the case `case` apart: its legend goes, its controls stay.
    fn ungroup(&mut self, case: u64) {
        self.items
            .retain(|i| !(i.kind == Kind::Legend && i.case == Some(case)));
        for i in &mut self.items {
            if i.case == Some(case) {
                i.case = None;
            }
        }
        if self.selected == Some(case) {
            self.selected = None;
        }
    }

    /// Removes the legends left with no control.
    fn drop_empty_cases(&mut self) {
        let empty: Vec<u64> = self
            .items
            .iter()
            .filter(|i| i.kind == Kind::Legend)
            .filter_map(|i| i.case)
            .filter(|&c| {
                !self
                    .items
                    .iter()
                    .any(|i| i.case == Some(c) && i.kind != Kind::Legend)
            })
            .collect();
        for c in empty {
            self.ungroup(c);
        }
    }

    /// Moves the item at `from` one step to `to`. A move across a case's
    /// edge first crosses the edge: the item joins or leaves the case in
    /// place (past a legend, it changes place too). Legends stay put.
    fn step(&mut self, from: usize, to: usize) {
        if self.items[from].kind == Kind::Legend {
            return;
        }
        let (mine, theirs) = (self.items[from].case, self.items[to].case);
        if self.items[to].kind == Kind::Legend {
            self.items.swap(from, to);
            self.items[to].case = if to > from {
                // Down past a legend: into its case, just below it.
                theirs
            } else {
                // Up past its case's legend: out of the case, into
                // whatever comes before it.
                to.checked_sub(1).and_then(|i| self.items[i].case)
            };
        } else if mine != theirs {
            self.items[from].case = theirs;
        } else {
            self.items.swap(from, to);
        }
        self.drop_empty_cases();
    }

    /// Lays the box out, keeping the trail.
    fn run_layout(&mut self, width: f32) -> (Layout, Vec<Vec<usize>>) {
        let unit = self.density.unit();
        let mut container = Container::new(width, unit);
        if self.bounded {
            container = container.with_height(self.box_height);
        }
        let (elements, forms): (Vec<_>, Vec<_>) = self.items.iter().map(Item::element).unzip();
        let previous = if self.hysteresis {
            self.last.as_ref()
        } else {
            None
        };
        let result = layout_grouped(&container, &elements, &self.groups(), previous);

        let choices: Vec<_> = result
            .placements
            .iter()
            .map(|p| (p.key, p.form, p.orientation))
            .collect();
        let flipped = !self.choices.is_empty() && choices != self.choices;
        let regridded = self
            .last
            .as_ref()
            .is_some_and(|l| l.columns != result.columns);
        if flipped {
            self.flips += 1;
        }
        if regridded {
            self.grid_changes += 1;
        }
        if width != self.last_width || flipped {
            if self.trail.len() == TRAIL {
                self.trail.pop_front();
            }
            self.trail.push_back(Sample {
                natural: width * TWELFTHS_PER_UNIT as f32 / unit,
                columns: result.columns,
                flipped,
            });
        }
        self.choices = choices;
        self.last_width = width;
        self.last = Some(result.clone());
        (result, forms)
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.themed == Some(self.density) {
            return;
        }
        let mut spec = egui_pop::theme::ThemeSpec::default();
        spec.density = match self.density {
            Density::Compact => egui_pop::prelude::Density::Compact,
            _ => egui_pop::prelude::Density::Comfortable,
        };
        egui_pop::theme::install(ctx, &spec);
        self.themed = Some(self.density);
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.apply_theme(ui.ctx());
        let time = ui.input(|i| i.time);
        let unit = self.density.unit();
        let width = if self.wobble {
            ui.ctx().request_repaint();
            self.box_width + (time * 2.0).sin() as f32 * unit * 0.3
        } else {
            self.box_width
        };
        let width = width.max(1.0);
        let (result, forms) = self.run_layout(width);

        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::left("palette")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.palette(ui, &result));
            });
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.inspector(ui, &result));
            });
        egui::Panel::bottom("dump")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.bottom(ui, &result));
            });
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                self.canvas(ui, &result, &forms, width, time);
            });
        });
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let unit = self.density.unit();
        ui.horizontal_wrapped(|ui| {
            ui.strong("egui_fit");
            ui.label(RichText::new(self.preset).weak());
            ui.separator();
            ui.label("Width");
            ui.add(
                egui::Slider::new(&mut self.box_width, 40.0..=2400.0)
                    .suffix(" pt")
                    .integer(),
            );
            ui.label(format!("= {:.2} units", self.box_width / unit));
            ui.separator();
            ui.label("Density");
            for d in Density::ALL {
                ui.selectable_value(&mut self.density, d, d.name())
                    .on_hover_text(format!("Unit {} pt", d.unit()));
            }
            ui.separator();
            ui.checkbox(&mut self.bounded, "Bounded height");
            if self.bounded {
                ui.add(
                    egui::Slider::new(&mut self.box_height, 20.0..=1200.0)
                        .suffix(" pt")
                        .integer(),
                );
            }
            ui.separator();
            ui.checkbox(&mut self.hysteresis, "Hysteresis")
                .on_hover_text(
                    "Pass the previous layout: the grid and the forms hold while \
                 the box stays within half a unit of a threshold",
                );
            ui.checkbox(&mut self.wobble, "Wobble").on_hover_text(
                "Swing the width by 0.3 unit around its value: with hysteresis, \
                 the layout does not flicker",
            );
        });
    }

    fn palette(&mut self, ui: &mut Ui, result: &Layout) {
        ui.heading("Presets");
        for p in PRESETS {
            if ui.selectable_label(self.preset == p.0, p.0).clicked() {
                self.preset(p);
            }
        }
        ui.separator();
        ui.heading("Add");
        ui.horizontal_wrapped(|ui| {
            for kind in Kind::ALL {
                if ui.button(kind.name()).on_hover_text(kind.hint()).clicked() {
                    let item = self.new_item(kind);
                    self.selected = Some(item.key);
                    self.items.push(item);
                }
            }
        });
        ui.separator();
        ui.heading("Elements");
        ui.label(RichText::new("In logical order: Tab, MIDI banks").weak());
        ui.horizontal_wrapped(|ui| {
            let n = self.marked.len();
            if ui
                .add_enabled(n > 0, egui::Button::new(format!("Group {n} as a case")))
                .on_hover_text(
                    "Tick elements below, then group them under a legend: \
                     packing stays dense inside the case, and nothing \
                     outside it moves into its gaps",
                )
                .clicked()
            {
                self.group_marked();
            }
            let case = self
                .selected
                .and_then(|k| self.index_of(k))
                .and_then(|i| self.items[i].case);
            if ui
                .add_enabled(case.is_some(), egui::Button::new("Ungroup"))
                .on_hover_text("Take the selected element's case apart")
                .clicked()
            {
                if let Some(c) = case {
                    self.ungroup(c);
                }
            }
        });
        let mut action = None;
        for (i, item) in self.items.iter().enumerate() {
            let placed = result.get(item.key);
            ui.horizontal(|ui| {
                let mut ticked = self.marked.contains(&item.key);
                if item.kind != Kind::Legend
                    && ui
                        .checkbox(&mut ticked, "")
                        .on_hover_text("Mark, to group into a case")
                        .changed()
                {
                    action = Some(Action::Mark(item.key, ticked));
                }
                if item.case.is_some() {
                    let bar = if item.kind == Kind::Legend {
                        "┌"
                    } else {
                        "│"
                    };
                    ui.label(RichText::new(bar).weak());
                }
                let letter = letter(i);
                let status = placed.map_or(Status::Placed, |p| p.status);
                ui.colored_label(status_color(&status), "●")
                    .on_hover_text(status_text(&status));
                let text = format!("{letter}  {}", item.label);
                if ui
                    .selectable_label(self.selected == Some(item.key), text)
                    .on_hover_text(item.kind.name())
                    .clicked()
                {
                    action = Some(Action::Select(item.key));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("x").on_hover_text("Remove").clicked() {
                        action = Some(Action::Remove(i));
                    }
                    if ui.small_button("v").on_hover_text("Move down").clicked() {
                        action = Some(Action::Move(i, i + 1));
                    }
                    if ui.small_button("^").on_hover_text("Move up").clicked() && i > 0 {
                        action = Some(Action::Move(i, i - 1));
                    }
                });
            });
        }
        self.act(action);
        ui.separator();
        ui.heading("Overlays");
        ui.checkbox(&mut self.overlays.grid, "Grid in twelfths");
        ui.checkbox(&mut self.overlays.status, "Slot status");
        ui.checkbox(&mut self.overlays.order, "Logical and visual order");
        ui.checkbox(&mut self.overlays.groups, "Case frames");
    }

    fn act(&mut self, action: Option<Action>) {
        match action {
            Some(Action::Select(key)) => {
                if self.selected != Some(key) {
                    self.form_tab = 0;
                }
                self.selected = Some(key);
            }
            Some(Action::Remove(i)) => {
                let item = self.items.remove(i);
                if item.kind == Kind::Legend {
                    if let Some(c) = item.case {
                        self.ungroup(c);
                    }
                }
                if self.selected == Some(item.key) {
                    self.selected = None;
                }
                self.marked.remove(&item.key);
                self.drop_empty_cases();
            }
            Some(Action::Mark(key, on)) => {
                if on {
                    self.marked.insert(key);
                } else {
                    self.marked.remove(&key);
                }
            }
            Some(Action::Move(from, to)) if to < self.items.len() => {
                self.step(from, to);
            }
            _ => {}
        }
    }

    fn canvas(
        &mut self,
        ui: &mut Ui,
        result: &Layout,
        forms: &[Vec<usize>],
        width: f32,
        time: f64,
    ) {
        let unit = self.density.unit();
        let height = if self.bounded {
            self.box_height
        } else {
            result.height_points().max(unit / 3.0)
        };
        ui.add_space(4.0);
        let (outer, _) = ui.allocate_exact_size(vec2(width + 24.0, height + 24.0), Sense::hover());
        let rect = Rect::from_min_size(outer.min + vec2(4.0, 4.0), vec2(width, height));
        let visuals = ui.visuals().clone();
        let painter = ui.painter().clone();
        painter.rect_filled(rect, 4.0, visuals.faint_bg_color);
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color),
            StrokeKind::Outside,
        );

        if self.overlays.grid {
            grid(&painter, rect, unit, result.columns, &visuals);
        }

        // Each case's frame, around the band the engine gives it.
        if self.overlays.groups {
            for g in &result.groups {
                let Some(b) = g.points else { continue };
                let frame = Rect::from_min_size(rect.min + vec2(b.x, b.y), vec2(b.w, b.h));
                painter.rect_filled(
                    frame.shrink(1.0),
                    6.0,
                    visuals.selection.bg_fill.gamma_multiply(0.08),
                );
                painter.rect_stroke(
                    frame.shrink(1.0),
                    6.0,
                    Stroke::new(1.0, visuals.selection.bg_fill.gamma_multiply(0.7)),
                    StrokeKind::Inside,
                );
            }
        }

        // Slot backgrounds first: a click on a slot's free part selects it.
        let mut select = None;
        for p in &result.placements {
            let Some(r) = slot(rect, p) else { continue };
            let response = ui.interact(r, ui.scope_id().with(("slot", p.key)), Sense::click());
            if response.clicked() {
                select = Some(p.key);
            }
            painter.rect_filled(r.shrink(0.5), 3.0, visuals.extreme_bg_color);
        }

        // The widgets.
        for (i, item) in self.items.iter_mut().enumerate() {
            let Some(p) = result.placements.get(i) else {
                continue;
            };
            let Some(r) = slot(rect, p) else { continue };
            let form = forms[i].get(p.form).copied().unwrap_or(0);
            render::show(ui, item, form, p.orientation, r.shrink(2.0), time);
        }

        // The overlays on top.
        let accent = visuals.selection.bg_fill;
        if self.overlays.order {
            let centres: Vec<_> = result
                .placements
                .iter()
                .filter_map(|p| slot(rect, p).map(|r| r.center()))
                .collect();
            painter.add(egui::Shape::dashed_line(
                &centres,
                Stroke::new(1.5, accent.gamma_multiply(0.8)),
                6.0,
                4.0,
            ));
        }
        for p in &result.placements {
            let Some(r) = slot(rect, p) else { continue };
            let selected = self.selected == Some(p.key);
            if selected {
                painter.rect_stroke(r, 3.0, Stroke::new(2.0, accent), StrokeKind::Inside);
            }
            if self.overlays.status {
                let color = status_color(&p.status);
                painter.rect_stroke(
                    r.shrink(0.5),
                    3.0,
                    Stroke::new(1.0, color.gamma_multiply(0.7)),
                    StrokeKind::Inside,
                );
                let dot = Rect::from_center_size(r.right_top() + vec2(-7.0, 7.0), vec2(10.0, 10.0));
                painter.circle(
                    dot.center(),
                    4.5,
                    color,
                    Stroke::new(1.0, Color32::from_black_alpha(160)),
                );
                let response = ui
                    .interact(dot, ui.scope_id().with(("status", p.key)), Sense::click())
                    .on_hover_text(placement_text(p, &self.items));
                if response.clicked() {
                    select = Some(p.key);
                }
            }
            if self.overlays.order {
                let visual = p.visual.map_or("-".to_string(), |v| (v + 1).to_string());
                let text = format!("L{} V{}", p.logical + 1, visual);
                let moved = Some(p.logical) != p.visual;
                let galley = painter.layout_no_wrap(text, FontId::monospace(10.0), Color32::WHITE);
                let at = r.left_top() + vec2(2.0, 2.0);
                let badge = Rect::from_min_size(at, galley.size() + vec2(6.0, 2.0));
                let fill = if moved {
                    Color32::from_rgb(170, 90, 30)
                } else {
                    Color32::from_black_alpha(170)
                };
                painter.rect_filled(badge, 3.0, fill);
                painter.galley(at + vec2(3.0, 1.0), galley, Color32::WHITE);
                ui.interact(badge, ui.scope_id().with(("order", p.key)), Sense::hover())
                    .on_hover_text(if moved {
                        "Logical order (Tab, MIDI) and visual order differ: \
                         a later element moved up into a gap"
                    } else {
                        "Logical order (Tab, MIDI) and visual order agree"
                    });
            }
        }

        // The right edge resizes the box; so does the bottom edge when bounded.
        let edge = Rect::from_min_max(
            pos2(rect.right() - 3.0, rect.top()),
            pos2(rect.right() + 9.0, rect.bottom()),
        );
        let drag = ui
            .interact(edge, ui.scope_id().with("right-edge"), Sense::drag())
            .on_hover_cursor(CursorIcon::ResizeHorizontal)
            .on_hover_text("Drag to resize");
        let handle = if drag.hovered() || drag.dragged() {
            accent
        } else {
            visuals.widgets.inactive.fg_stroke.color
        };
        painter.line_segment(
            [
                pos2(rect.right() + 3.0, rect.center().y - 16.0),
                pos2(rect.right() + 3.0, rect.center().y + 16.0),
            ],
            Stroke::new(3.0, handle),
        );
        if drag.dragged() {
            self.box_width = (self.box_width + drag.drag_delta().x).clamp(40.0, 2400.0);
        }
        if self.bounded {
            let edge = Rect::from_min_max(
                pos2(rect.left(), rect.bottom() - 3.0),
                pos2(rect.right(), rect.bottom() + 9.0),
            );
            let drag = ui
                .interact(edge, ui.scope_id().with("bottom-edge"), Sense::drag())
                .on_hover_cursor(CursorIcon::ResizeVertical);
            painter.line_segment(
                [
                    pos2(rect.center().x - 16.0, rect.bottom() + 3.0),
                    pos2(rect.center().x + 16.0, rect.bottom() + 3.0),
                ],
                Stroke::new(3.0, handle),
            );
            if drag.dragged() {
                self.box_height = (self.box_height + drag.drag_delta().y).clamp(20.0, 1200.0);
            }
        }

        // What did not fit.
        let overflow: Vec<_> = result
            .placements
            .iter()
            .filter(|p| !p.status.is_placed())
            .collect();
        if !overflow.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("Overflow: not placed, for the caller to fold or show elsewhere")
                    .strong(),
            );
            for p in overflow {
                let label = self.items.get(p.logical).map_or("", |i| i.label.as_str());
                let text = format!(
                    "{}  {}: {}",
                    letter(p.logical),
                    label,
                    status_text(&p.status)
                );
                if ui
                    .selectable_label(
                        self.selected == Some(p.key),
                        RichText::new(text).color(status_color(&p.status)),
                    )
                    .clicked()
                {
                    select = Some(p.key);
                }
            }
        }
        if let Some(key) = select {
            self.act(Some(Action::Select(key)));
        }
    }

    fn inspector(&mut self, ui: &mut Ui, result: &Layout) {
        ui.heading("Selected element");
        let Some(i) = self.selected.and_then(|k| self.index_of(k)) else {
            ui.label("Click a slot, or an element in the list.");
            return;
        };
        let columns = result.columns;
        let placement = result.get(self.items[i].key).copied();
        let mut action = None;
        let item = &mut self.items[i];
        ui.horizontal(|ui| {
            ui.label(format!("{}  {}", letter(i), item.kind.name()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Remove").clicked() {
                    action = Some(Action::Remove(i));
                }
                if ui.button("Down").clicked() {
                    action = Some(Action::Move(i, i + 1));
                }
                if ui.button("Up").clicked() && i > 0 {
                    action = Some(Action::Move(i, i - 1));
                }
            });
        });
        ui.label(RichText::new(item.kind.hint()).weak());
        ui.horizontal(|ui| {
            ui.label("Label");
            ui.text_edit_singleline(&mut item.label);
        });
        if let Some(p) = placement {
            ui.label(
                placement_text(&p, std::slice::from_ref(item))
                    .replace(&format!("{}: ", item.label), ""),
            );
        }

        ui.separator();
        ui.strong("Forms");
        ui.horizontal(|ui| {
            for (f, form) in item.forms.iter().enumerate() {
                let name = FORM_NAMES.get(f).copied().unwrap_or("smaller");
                let text = if form.enabled || f == 0 {
                    name.to_string()
                } else {
                    format!("({name})")
                };
                ui.selectable_value(&mut self.form_tab, f, text);
            }
        });
        let f = self.form_tab.min(item.forms.len() - 1);
        if f > 0 {
            ui.checkbox(
                &mut item.forms[f].enabled,
                "Declared: the engine may step down to it",
            );
        }
        form_editor(ui, &mut item.forms[f], (item.key, f));

        ui.separator();
        ui.strong("Orientations");
        ui.horizontal(|ui| {
            ui.checkbox(&mut item.horizontal, "Horizontal");
            ui.checkbox(&mut item.vertical, "Vertical");
        });
        if item.horizontal && item.vertical {
            ui.checkbox(&mut item.prefer_vertical, "Prefer vertical");
        }
        ui.horizontal(|ui| {
            ui.label("Pin");
            ui.selectable_value(&mut item.pinned_orientation, None, "none");
            ui.selectable_value(
                &mut item.pinned_orientation,
                Some(Orientation::Horizontal),
                "horizontal",
            );
            ui.selectable_value(
                &mut item.pinned_orientation,
                Some(Orientation::Vertical),
                "vertical",
            );
        });

        ui.separator();
        ui.strong("Position pin");
        let mut pinned = item.pin.is_some();
        if ui
            .checkbox(&mut pinned, "Pinned: placed before the flow")
            .changed()
        {
            item.pin = if pinned {
                Some(
                    placement
                        .and_then(pin_of)
                        .unwrap_or(Cell { col: 0, row: 0 }),
                )
            } else {
                None
            };
        }
        if let Some(pin) = &mut item.pin {
            ui.horizontal(|ui| {
                ui.label("Column");
                ui.add(egui::DragValue::new(&mut pin.col).range(0..=columns.max(1)));
                ui.label("Row");
                ui.add(egui::DragValue::new(&mut pin.row).range(0..=1200));
                ui.label("twelfths");
            });
            ui.label(RichText::new("Counted from the top of its band: its case's controls, or its run of loose ones").weak());
            if ui.button("Pin where it is now").clicked() {
                if let Some(at) = placement.and_then(pin_of) {
                    *pin = at;
                }
            }
        }
        self.act(action);
    }

    fn bottom(&mut self, ui: &mut Ui, result: &Layout) {
        egui::CollapsingHeader::new("Hysteresis trail")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(format!(
                    "Grid width changes: {}   Form or orientation flips: {}   ({})",
                    self.grid_changes,
                    self.flips,
                    if self.hysteresis {
                        "hysteresis on"
                    } else {
                        "hysteresis off"
                    }
                ));
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        self.trail.clear();
                        self.flips = 0;
                        self.grid_changes = 0;
                    }
                    ui.label(
                        RichText::new(
                            "Grey: the box width. Accent: the grid width kept. \
                             Red ticks: a form or orientation changed.",
                        )
                        .weak(),
                    );
                });
                trail(ui, &self.trail, self.density.unit());
            });
        egui::CollapsingHeader::new("dump()")
            .default_open(true)
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(result.dump()).monospace()).selectable(true));
            });
    }
}

#[derive(Clone, Copy, Debug)]
enum Action {
    Select(u64),
    Remove(usize),
    Move(usize, usize),
    Mark(u64, bool),
}

/// Where a placed element is, as a pin: from the top of its band.
fn pin_of(p: Placement) -> Option<Cell> {
    p.rect.map(|r| Cell {
        col: r.x,
        row: r.y - p.band_top,
    })
}

/// The dump's letter for a logical index.
fn letter(i: usize) -> char {
    const LABELS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    LABELS.get(i).map_or('#', |&b| b as char)
}

/// A placement's slot in the box, in screen points.
fn slot(rect: Rect, p: &Placement) -> Option<Rect> {
    let r = p.points?;
    Some(Rect::from_min_size(
        rect.min + vec2(r.x, r.y),
        vec2(r.w, r.h),
    ))
}

fn status_color(status: &Status) -> Color32 {
    match status {
        Status::Placed => Color32::from_rgb(90, 180, 100),
        Status::Unpinned(_) => Color32::from_rgb(225, 170, 60),
        Status::Overflow(_) => Color32::from_rgb(220, 80, 70),
    }
}

fn reason_text(reason: Reason) -> &'static str {
    match reason {
        Reason::NoForms => "it declares no form",
        Reason::TooNarrow => "even its smallest form is wider than the box",
        Reason::TooShort => "even its smallest form is taller than the bounded box",
        Reason::NoRoom => "it fits the box's shape, but the bounded box is full",
        Reason::PinOutOfBounds => "its pin puts it outside the box",
        Reason::PinOverlap => "its pin overlaps an element pinned before it",
    }
}

fn status_text(status: &Status) -> String {
    match status {
        Status::Placed => "placed".to_string(),
        Status::Unpinned(r) => format!("unpinned ({r}): {}", reason_text(*r)),
        Status::Overflow(r) => format!("overflow ({r}): {}", reason_text(*r)),
    }
}

fn placement_text(p: &Placement, items: &[Item]) -> String {
    let label = items
        .iter()
        .find(|i| i.key == p.key)
        .map_or("", |i| i.label.as_str());
    let form = FORM_NAMES.get(p.form).copied().unwrap_or("smaller");
    let mut text = format!(
        "{label}: {}, form {} ({form}), {}",
        status_text(&p.status),
        p.form,
        p.orientation
    );
    if let Some(r) = p.rect {
        text.push_str(&format!(
            "\nslot {} x {} at {},{} (twelfths)",
            size_text(r.w),
            size_text(r.h),
            r.x,
            r.y
        ));
    }
    if p.group.is_some() {
        text.push_str("\nin a case");
    }
    if p.band_top > 0 {
        text.push_str(&format!("\nits band starts at row {}", p.band_top));
    }
    text
}

/// A length in twelfths as a slot size: ¼, ⅓, ½, ⅔, ¾ or whole units.
fn size_text(t: u32) -> String {
    match t {
        3 => "¼".into(),
        4 => "⅓".into(),
        6 => "½".into(),
        8 => "⅔".into(),
        9 => "¾".into(),
        t if t % TWELFTHS_PER_UNIT == 0 => (t / TWELFTHS_PER_UNIT).to_string(),
        t => format!("{t}/12"),
    }
}

/// The grid: every twelfth faint, every unit stronger, the grid's own
/// width (which hysteresis may hold below the box's) marked.
fn grid(painter: &egui::Painter, rect: Rect, unit: f32, columns: u32, visuals: &egui::Visuals) {
    let step = unit / TWELFTHS_PER_UNIT as f32;
    let base = visuals.widgets.noninteractive.bg_stroke.color;
    let faint = Stroke::new(0.5, base.gamma_multiply(0.35));
    let strong = Stroke::new(1.0, base.gamma_multiply(0.9));
    let mut i = 0;
    while i as f32 * step <= rect.width() {
        let x = rect.left() + i as f32 * step;
        let stroke = if i % TWELFTHS_PER_UNIT == 0 {
            strong
        } else {
            faint
        };
        painter.vline(x, rect.y_range(), stroke);
        i += 1;
    }
    let mut j = 0;
    while j as f32 * step <= rect.height() {
        let y = rect.top() + j as f32 * step;
        let stroke = if j % TWELFTHS_PER_UNIT == 0 {
            strong
        } else {
            faint
        };
        painter.hline(rect.x_range(), y, stroke);
        j += 1;
    }
    let right = rect.left() + columns as f32 * step;
    painter.vline(
        right,
        rect.y_range(),
        Stroke::new(1.5, visuals.selection.bg_fill.gamma_multiply(0.7)),
    );
}

/// The trail: the box width and the grid width over the last samples.
fn trail(ui: &mut Ui, samples: &VecDeque<Sample>, unit: f32) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width().max(200.0), 90.0), Sense::hover());
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 3.0, visuals.extreme_bg_color);
    if samples.len() < 2 {
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Resize the box, or turn on Wobble",
            FontId::proportional(12.0),
            visuals.weak_text_color(),
        );
        return;
    }
    let (lo, hi) = samples.iter().fold((f32::MAX, f32::MIN), |(lo, hi), s| {
        (
            lo.min(s.natural).min(s.columns as f32),
            hi.max(s.natural).max(s.columns as f32),
        )
    });
    let pad = 2.0;
    let (lo, hi) = (lo - pad, (hi + pad).max(lo + 1.0));
    let x = |i: usize| rect.left() + rect.width() * i as f32 / (TRAIL - 1) as f32;
    let y = |v: f32| rect.bottom() - rect.height() * (v - lo) / (hi - lo);
    // Unit thresholds.
    let half = TWELFTHS_PER_UNIT as f32 / 2.0;
    let mut t = (lo / half).ceil() * half;
    while t <= hi {
        let unit_line = (t as u32).is_multiple_of(TWELFTHS_PER_UNIT);
        let color = visuals.widgets.noninteractive.bg_stroke.color;
        painter.hline(
            rect.x_range(),
            y(t),
            Stroke::new(if unit_line { 1.0 } else { 0.5 }, color.gamma_multiply(0.6)),
        );
        if unit_line {
            painter.text(
                pos2(rect.left() + 2.0, y(t)),
                Align2::LEFT_BOTTOM,
                format!(
                    "{} u ({:.0} pt)",
                    t as u32 / TWELFTHS_PER_UNIT,
                    t / 12.0 * unit
                ),
                FontId::monospace(9.0),
                visuals.weak_text_color(),
            );
        }
        t += half;
    }
    for (i, s) in samples.iter().enumerate() {
        if s.flipped {
            painter.vline(
                x(i),
                rect.y_range(),
                Stroke::new(1.0, Color32::from_rgb(220, 80, 70)),
            );
        }
    }
    let natural: Vec<_> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| pos2(x(i), y(s.natural)))
        .collect();
    let columns: Vec<_> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| pos2(x(i), y(s.columns as f32)))
        .collect();
    painter.line(natural, Stroke::new(1.0, visuals.weak_text_color()));
    painter.line(columns, Stroke::new(2.0, visuals.selection.bg_fill));
}

/// The size policy editor of one form.
fn form_editor(ui: &mut Ui, form: &mut Form, salt: (u64, usize)) {
    let p = &mut form.policy;
    ui.horizontal(|ui| {
        ui.label("Growth");
        ui.selectable_value(&mut p.grow, Grow::Fixed, "fixed");
        ui.selectable_value(&mut p.grow, Grow::Along(Axis::Width), "along width");
        ui.selectable_value(&mut p.grow, Grow::Along(Axis::Height), "along height");
    });
    egui::Grid::new(("policy", salt))
        .num_columns(3)
        .spacing(vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label("");
            ui.label("width");
            ui.label("height");
            ui.end_row();
            for (name, size) in [
                ("min", &mut p.min_size),
                ("preferred", &mut p.size),
                ("max", &mut p.max_size),
            ] {
                ui.label(name);
                size_combo(ui, &mut size.width, (salt, name, 0));
                size_combo(ui, &mut size.height, (salt, name, 1));
                ui.end_row();
            }
        });
    let mut locked = p.aspect_ratio.is_some();
    ui.horizontal(|ui| {
        if ui.checkbox(&mut locked, "Aspect ratio").changed() {
            p.aspect_ratio = locked.then_some(AspectRatio {
                width: 1,
                height: 1,
            });
        }
        if let Some(r) = &mut p.aspect_ratio {
            ui.add(egui::DragValue::new(&mut r.width).range(1..=12));
            ui.label(":");
            ui.add(egui::DragValue::new(&mut r.height).range(1..=12));
        }
    });
    if p.min_size.width > p.size.width || p.min_size.height > p.size.height {
        ui.label(
            RichText::new("min is above preferred: the engine raises preferred to min").weak(),
        );
    }
}

/// A drop-down of the allowed sizes.
fn size_combo(ui: &mut Ui, size: &mut Size, salt: impl std::hash::Hash + std::fmt::Debug) {
    egui::ComboBox::from_id_salt(egui::IdSalt::new(salt))
        .width(56.0)
        .selected_text(size_text(size.twelfths()))
        .show_ui(ui, |ui| {
            for t in SIZES {
                if let Some(s) = Size::from_twelfths(t) {
                    ui.selectable_value(size, s, size_text(t));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo(preset: &str) -> Demo {
        let mut d = Demo::default();
        let p = PRESETS.iter().find(|p| p.0 == preset).copied().unwrap();
        d.preset(p);
        d
    }

    fn labels(d: &Demo) -> Vec<(&str, Option<u64>)> {
        d.items.iter().map(|i| (i.label.as_str(), i.case)).collect()
    }

    #[test]
    fn the_two_cases_preset_frames_two_groups() {
        let mut d = demo("Two cases");
        let groups = d.groups();
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|g| g.header));
        let (l, _) = d.run_layout(4.0 * d.density.unit());
        let bands: Vec<_> = l.groups.iter().map(|g| g.rect.unwrap()).collect();
        assert!(bands[0].bottom() <= bands[1].y);
        // The loose knob between the cases sits between their bands.
        let knob = l.placements[7].rect.unwrap();
        assert!(bands[0].bottom() <= knob.y && knob.bottom() <= bands[1].y);
    }

    #[test]
    fn marked_items_group_under_a_legend_and_ungroup() {
        let mut d = demo("Darkroom card");
        let (a, c) = (d.items[1].key, d.items[3].key);
        d.marked.extend([c, a]);
        d.group_marked();
        let case = d.items[1].key;
        assert_eq!(
            labels(&d)[..5],
            [
                ("Enabled", None),
                ("Case 1", Some(case)),
                ("Mode", Some(case)),
                ("Contrast", Some(case)),
                ("Exposure", None)
            ]
        );
        assert_eq!(d.groups(), [Group::new(case, 1..4).with_header()]);
        d.ungroup(case);
        assert_eq!(d.items.len(), 9);
        assert!(d.groups().is_empty());
    }

    #[test]
    fn moves_cross_a_case_edge_first() {
        let mut d = demo("Darkroom card");
        d.marked.insert(d.items[1].key);
        d.group_marked();
        let case = d.items[1].key;
        // Enabled moves down past the legend: into the case.
        d.step(0, 1);
        assert_eq!(
            labels(&d)[..3],
            [
                ("Case 1", Some(case)),
                ("Enabled", Some(case)),
                ("Mode", Some(case))
            ]
        );
        // Mode moves down onto Exposure: out of the case, in place.
        d.step(2, 3);
        assert_eq!(labels(&d)[2], ("Mode", None));
        // Enabled moves up past the legend: out; the empty case goes.
        d.step(1, 0);
        assert!(d
            .items
            .iter()
            .all(|i| i.case.is_none() && i.kind != Kind::Legend));
    }
}

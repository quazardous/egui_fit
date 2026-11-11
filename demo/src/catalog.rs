//! The kinds of element the demo offers, what each declares to the engine
//! (its forms and orientations), and the values each widget edits.

use egui_fit::{Axis, Cell, Element, Orientation, Size, Size2, SizePolicy};
use egui_pop::viewport::wheel::WheelValue;

/// The names of the forms, by index.
pub const FORM_NAMES: [&str; 3] = ["full", "compact", "simplified"];

/// What an element shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Slider,
    Toggle,
    Dropdown,
    Field,
    Knob,
    Curve,
    Wheels,
    Meter,
    Histogram,
    Block,
    /// A case's legend: its group's header. Made by grouping, not in the
    /// palette.
    Legend,
}

impl Kind {
    /// Every kind, in the palette's order.
    pub const ALL: [Kind; 10] = [
        Kind::Slider,
        Kind::Toggle,
        Kind::Dropdown,
        Kind::Field,
        Kind::Knob,
        Kind::Curve,
        Kind::Wheels,
        Kind::Meter,
        Kind::Histogram,
        Kind::Block,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Slider => "Slider",
            Kind::Toggle => "Toggle",
            Kind::Dropdown => "Drop-down",
            Kind::Field => "Value field",
            Kind::Knob => "Knob",
            Kind::Curve => "Curve",
            Kind::Wheels => "Colour wheels",
            Kind::Meter => "Meter",
            Kind::Histogram => "Histogram",
            Kind::Block => "Plain block",
            Kind::Legend => "Legend",
        }
    }

    /// What the palette says of it.
    pub fn hint(self) -> &'static str {
        match self {
            Kind::Slider => "Growable along its length, horizontal or vertical",
            Kind::Toggle => "A quarter of a unit",
            Kind::Dropdown => "Half a unit",
            Kind::Field => "A number with its unit",
            Kind::Knob => "Fixed; egui-pop has no knob, so this is a small fixed slider",
            Kind::Curve => "A block whose aspect ratio is locked to 1:1",
            Kind::Wheels => "Lift, gamma and gain: a block, turned when narrow",
            Kind::Meter => "An indicator, horizontal or vertical",
            Kind::Histogram => "An indicator block, growable",
            Kind::Block => "A plain box, for packing experiments",
            Kind::Legend => "A case's title: its group's header, on a band of its own",
        }
    }

    /// The forms it declares: full, compact, simplified (some kinds have
    /// fewer).
    pub fn forms(self) -> Vec<SizePolicy> {
        use Size as S;
        let third = S::THIRD;
        let quarter = S::QUARTER;
        match self {
            Kind::Slider => vec![
                SizePolicy::growable(
                    s(S::ONE, third),
                    s(S::units(2), third),
                    s(S::units(6), third),
                    Axis::Width,
                ),
                SizePolicy::growable(
                    s(S::ONE, quarter),
                    s(S::ONE, quarter),
                    s(S::units(4), quarter),
                    Axis::Width,
                ),
                SizePolicy::fixed(s(S::THREE_QUARTERS, quarter)),
            ],
            Kind::Toggle => vec![
                SizePolicy::fixed(s(quarter, third)),
                SizePolicy::fixed(s(quarter, quarter)),
            ],
            Kind::Dropdown => vec![
                SizePolicy::fixed(s(S::HALF, third)),
                SizePolicy::fixed(s(third, third)),
            ],
            Kind::Field => vec![
                SizePolicy::fixed(s(S::ONE, third)),
                SizePolicy::fixed(s(S::TWO_THIRDS, third)),
                SizePolicy::fixed(s(S::HALF, quarter)),
            ],
            Kind::Knob => vec![
                SizePolicy::fixed(s(S::ONE, third)),
                SizePolicy::fixed(s(S::THREE_QUARTERS, quarter)),
            ],
            Kind::Curve => vec![
                SizePolicy::growable(
                    s(S::ONE, S::ONE),
                    s(S::ONE, S::ONE),
                    s(S::units(3), S::units(3)),
                    Axis::Width,
                )
                .with_aspect_ratio(1, 1),
                SizePolicy::fixed(s(S::TWO_THIRDS, S::TWO_THIRDS)).with_aspect_ratio(1, 1),
                SizePolicy::fixed(s(S::HALF, S::HALF)).with_aspect_ratio(1, 1),
            ],
            Kind::Wheels => vec![
                SizePolicy::fixed(s(S::units(3), S::ONE)),
                SizePolicy::fixed(s(S::units(2), S::TWO_THIRDS)),
                SizePolicy::fixed(s(S::ONE, quarter)),
            ],
            Kind::Meter => vec![
                SizePolicy::growable(
                    s(S::ONE, quarter),
                    s(S::ONE, quarter),
                    s(S::units(4), quarter),
                    Axis::Width,
                ),
                SizePolicy::fixed(s(S::HALF, quarter)),
            ],
            Kind::Histogram => vec![
                SizePolicy::growable(
                    s(S::units(2), S::TWO_THIRDS),
                    s(S::units(2), S::TWO_THIRDS),
                    s(S::units(6), S::TWO_THIRDS),
                    Axis::Width,
                ),
                SizePolicy::fixed(s(S::ONE, third)),
            ],
            Kind::Block => vec![SizePolicy::fixed(s(S::ONE, S::HALF))],
            // Alone on its band, it grows across the box.
            Kind::Legend => vec![SizePolicy::growable(
                s(S::QUARTER, quarter),
                s(S::ONE, quarter),
                s(S::units(20), quarter),
                Axis::Width,
            )],
        }
    }

    /// Its supported orientations, preferred first.
    pub fn orientations(self) -> &'static [Orientation] {
        match self {
            Kind::Slider | Kind::Wheels | Kind::Meter => {
                &[Orientation::Horizontal, Orientation::Vertical]
            }
            _ => &[Orientation::Horizontal],
        }
    }

    /// A default label.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Slider => "Exposure",
            Kind::Toggle => "Enabled",
            Kind::Dropdown => "Mode",
            Kind::Field => "Offset",
            Kind::Knob => "Gain",
            Kind::Curve => "Tone curve",
            Kind::Wheels => "Grading",
            Kind::Meter => "Level",
            Kind::Histogram => "Histogram",
            Kind::Block => "Block",
            Kind::Legend => "Case",
        }
    }
}

/// A size from its width and height.
pub const fn s(width: Size, height: Size) -> Size2 {
    Size2::new(width, height)
}

/// One declared form: smaller forms can be switched off.
#[derive(Clone, Debug)]
pub struct Form {
    pub enabled: bool,
    pub policy: SizePolicy,
}

/// What a widget edits. It lives in the item, not in the form, so it
/// survives a change of form or orientation.
#[derive(Clone, Debug)]
pub struct Values {
    pub number: f64,
    pub on: bool,
    pub choice: usize,
    pub curve: Vec<[f32; 2]>,
    pub wheels: [WheelValue; 3],
}

impl Default for Values {
    fn default() -> Self {
        Values {
            number: 50.0,
            on: true,
            choice: 0,
            curve: vec![[0.0, 0.0], [0.3, 0.22], [0.7, 0.8], [1.0, 1.0]],
            wheels: [WheelValue::default(); 3],
        }
    }
}

/// One element of the demo: what it declares and the values it edits.
#[derive(Clone, Debug)]
pub struct Item {
    pub key: u64,
    pub kind: Kind,
    pub label: String,
    pub forms: Vec<Form>,
    /// Supported orientations.
    pub horizontal: bool,
    pub vertical: bool,
    /// Vertical first when both are supported.
    pub prefer_vertical: bool,
    pub pinned_orientation: Option<Orientation>,
    /// The pin, counted from the top of the element's band.
    pub pin: Option<Cell>,
    /// The case it is in: the key of the case's legend.
    pub case: Option<u64>,
    pub values: Values,
}

impl Item {
    pub fn new(key: u64, kind: Kind) -> Item {
        let orientations = kind.orientations();
        Item {
            key,
            kind,
            label: kind.label().to_string(),
            forms: kind
                .forms()
                .into_iter()
                .map(|policy| Form {
                    enabled: true,
                    policy,
                })
                .collect(),
            horizontal: orientations.contains(&Orientation::Horizontal),
            vertical: orientations.contains(&Orientation::Vertical),
            prefer_vertical: false,
            pinned_orientation: None,
            pin: None,
            case: None,
            values: Values::default(),
        }
    }

    pub fn labelled(mut self, label: &str) -> Item {
        self.label = label.to_string();
        self
    }

    /// Only the full form, sized `size`, fixed.
    pub fn sized(mut self, size: Size2) -> Item {
        self.forms.truncate(1);
        self.forms[0].policy = SizePolicy::fixed(size);
        self
    }

    /// The same item in case `case`.
    pub fn in_case(mut self, case: u64) -> Item {
        self.case = Some(case);
        self
    }

    pub fn with_number(mut self, number: f64) -> Item {
        self.values.number = number;
        self
    }

    /// The element handed to the engine, and the declared form index of
    /// each form it lists (smaller forms switched off are left out).
    pub fn element(&self) -> (Element, Vec<usize>) {
        let enabled: Vec<usize> = self
            .forms
            .iter()
            .enumerate()
            .filter(|(i, f)| *i == 0 || f.enabled)
            .map(|(i, _)| i)
            .collect();
        let mut orientations = Vec::new();
        let (h, v) = (Orientation::Horizontal, Orientation::Vertical);
        match (self.horizontal, self.vertical, self.prefer_vertical) {
            (true, true, true) => orientations.extend([v, h]),
            (true, true, false) => orientations.extend([h, v]),
            (false, true, _) => orientations.push(v),
            _ => orientations.push(h),
        }
        let element = Element {
            key: self.key,
            forms: enabled.iter().map(|&i| self.forms[i].policy).collect(),
            orientations,
            pinned_orientation: self.pinned_orientation,
            pin: self.pin,
        };
        (element, enabled)
    }
}

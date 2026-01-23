#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Float,
    Int,
    Bool,
    Vec2,
    Vec3,
    String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamWidget {
    Default,
    Slider,
    Combo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamRange {
    Float { min: f32, max: f32 },
    Int { min: i32, max: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamOption {
    Int { value: i32, label: &'static str },
    String { value: &'static str, label: &'static str },
}

#[derive(Debug, Clone)]
pub struct ParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: ParamKind,
    pub widget: ParamWidget,
    pub range: Option<ParamRange>,
    pub options: Vec<ParamOption>,
    pub help: Option<&'static str>,
}

impl ParamSpec {
    pub fn new(key: &'static str, label: &'static str, kind: ParamKind) -> Self {
        Self {
            key,
            label,
            kind,
            widget: ParamWidget::Default,
            range: None,
            options: Vec::new(),
            help: None,
        }
    }

    pub fn float(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Float)
    }

    pub fn float_slider(key: &'static str, label: &'static str, min: f32, max: f32) -> Self {
        Self::new(key, label, ParamKind::Float).with_range(ParamRange::Float { min, max }, true)
    }

    pub fn int(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Int)
    }

    pub fn int_slider(key: &'static str, label: &'static str, min: i32, max: i32) -> Self {
        Self::new(key, label, ParamKind::Int).with_range(ParamRange::Int { min, max }, true)
    }

    pub fn int_enum(
        key: &'static str,
        label: &'static str,
        options: Vec<(i32, &'static str)>,
    ) -> Self {
        let options = options
            .into_iter()
            .map(|(value, label)| ParamOption::Int { value, label })
            .collect();
        Self::new(key, label, ParamKind::Int)
            .with_options(options, true)
    }

    pub fn bool(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Bool)
    }

    pub fn vec2(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Vec2)
    }

    pub fn vec3(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::Vec3)
    }

    pub fn string(key: &'static str, label: &'static str) -> Self {
        Self::new(key, label, ParamKind::String)
    }

    pub fn string_enum(
        key: &'static str,
        label: &'static str,
        options: Vec<(&'static str, &'static str)>,
    ) -> Self {
        let options = options
            .into_iter()
            .map(|(value, label)| ParamOption::String { value, label })
            .collect();
        Self::new(key, label, ParamKind::String)
            .with_options(options, true)
    }

    pub fn with_help(mut self, help: &'static str) -> Self {
        self.help = Some(help);
        self
    }

    pub fn with_widget(mut self, widget: ParamWidget) -> Self {
        self.widget = widget;
        self
    }

    pub fn with_range(mut self, range: ParamRange, slider: bool) -> Self {
        self.range = Some(range);
        if slider {
            self.widget = ParamWidget::Slider;
        }
        self
    }

    pub fn with_options(mut self, options: Vec<ParamOption>, combo: bool) -> Self {
        self.options = options;
        if combo {
            self.widget = ParamWidget::Combo;
        }
        self
    }
}

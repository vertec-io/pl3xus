use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    #[default]
    Industrial,
    Indigo,
    Emerald,
    Rose,
    Teal,
    Cyber,
    Amber,
    Slate,
    Crimson,
    Gold,
    DeepOcean,
    Arctic,
    Obsidian,
    Neon,
    Retro,
    Mist,
    Solar,
    Space,
    Light,
    Blood,
    Coffee,
    GlassDark,
}

impl Theme {
    pub fn class_name(&self) -> &'static str {
        match self {
            Theme::Industrial => "", // Default uses :root variables
            Theme::Indigo => "theme-indigo",
            Theme::Emerald => "theme-emerald",
            Theme::Rose => "theme-rose",
            Theme::Teal => "theme-teal",
            Theme::Cyber => "theme-cyber",
            Theme::Amber => "theme-amber",
            Theme::Slate => "theme-slate",
            Theme::Crimson => "theme-crimson",
            Theme::Gold => "theme-gold",
            Theme::DeepOcean => "theme-ocean",
            Theme::Arctic => "theme-arctic",
            Theme::Obsidian => "theme-obsidian",
            Theme::Neon => "theme-neon",
            Theme::Retro => "theme-retro",
            Theme::Mist => "theme-mist",
            Theme::Solar => "theme-solar",
            Theme::Space => "theme-space",
            Theme::Light => "theme-light",
            Theme::Blood => "theme-blood",
            Theme::Coffee => "theme-coffee",
            Theme::GlassDark => "theme-glass-dark",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Theme::Industrial => "Industrial Dark",
            Theme::Indigo => "Indigo Knight",
            Theme::Emerald => "Emerald Grove",
            Theme::Rose => "Rose Carbon",
            Theme::Teal => "Midnight Teal",
            Theme::Cyber => "Cyberpunk",
            Theme::Amber => "Amber Glow",
            Theme::Slate => "Slate Blue",
            Theme::Crimson => "Crimson Peak",
            Theme::Gold => "Golden Hour",
            Theme::DeepOcean => "Deep Ocean",
            Theme::Arctic => "Arctic Frost",
            Theme::Obsidian => "Obsidian Sharp",
            Theme::Neon => "Neon Pulse",
            Theme::Retro => "Retro Slate",
            Theme::Mist => "Forest Mist",
            Theme::Solar => "Solar Flare",
            Theme::Space => "Deep Space",
            Theme::Light => "Industrial Light",
            Theme::Blood => "Blood Moon",
            Theme::Coffee => "Coffee Bean",
            Theme::GlassDark => "Glass Dark",
        }
    }

    pub fn preview_color(&self) -> &'static str {
        match self {
            Theme::Industrial => "#00d9ff",  // Cyan
            Theme::Indigo => "#818cf8",      // Electric indigo
            Theme::Emerald => "#34d399",     // Green terminal
            Theme::Rose => "#fb7185",        // Elegant pink
            Theme::Teal => "#2dd4bf",        // Ocean teal
            Theme::Cyber => "#e879f9",       // Magenta neon
            Theme::Amber => "#fbbf24",       // Warm orange
            Theme::Slate => "#60a5fa",       // Professional blue
            Theme::Crimson => "#f87171",     // Deep red
            Theme::Gold => "#facc15",        // Luxurious gold
            Theme::DeepOcean => "#38bdf8",   // Teal highlight
            Theme::Arctic => "#3b82f6",      // Frost blue
            Theme::Obsidian => "#ffffff",    // Pure white
            Theme::Neon => "#c084fc",        // Violet
            Theme::Retro => "#f97316",       // Warm orange
            Theme::Mist => "#4ade80",        // Soft green
            Theme::Solar => "#fb923c",       // Orange glow
            Theme::Space => "#a78bfa",       // Purple
            Theme::Light => "#06b6d4",       // Cyan
            Theme::Blood => "#ef4444",       // Crimson
            Theme::Coffee => "#d97706",      // Warm brown
            Theme::GlassDark => "#60a5fa",   // Blue glass
        }
    }

    pub fn all() -> &'static [Theme] {
        &[
            Theme::Industrial,
            Theme::Indigo,
            Theme::Emerald,
            Theme::Rose,
            Theme::Teal,
            Theme::Cyber,
            Theme::Amber,
            Theme::Slate,
            Theme::Crimson,
            Theme::Gold,
            Theme::DeepOcean,
            Theme::Arctic,
            Theme::Obsidian,
            Theme::Neon,
            Theme::Retro,
            Theme::Mist,
            Theme::Solar,
            Theme::Space,
            Theme::Light,
            Theme::Blood,
            Theme::Coffee,
            Theme::GlassDark,
        ]
    }
}

#[derive(Clone, Copy)]
pub struct ThemeContext {
    pub theme: RwSignal<Theme>,
}

pub fn provide_theme_context() {
    let storage = window().local_storage().ok().flatten();
    
    // Load initial theme from localStorage
    let initial_theme: Theme = storage.as_ref()
        .and_then(|s| s.get_item("app-theme").ok().flatten())
        .and_then(|t| serde_json::from_str::<Theme>(&format!("\"{}\"", t)).ok())
        .unwrap_or_default();

    let theme = RwSignal::new(initial_theme);

    // Effect to persist theme and update body class
    Effect::new(move |_| {
        let current_theme = theme.get();
        let class = current_theme.class_name();
        
        // Update DOM
        if let Some(body) = document().body() {
            // Remove all possible theme classes
            for t in Theme::all() {
                let tc = t.class_name();
                if !tc.is_empty() {
                    let _ = body.class_list().remove_1(tc);
                }
            }
            // Add new theme class
            if !class.is_empty() {
                let _ = body.class_list().add_1(class);
            }
        }

        // Persist to localStorage
        if let Some(s) = &storage {
            let serialized = serde_json::to_string(&current_theme).unwrap_or_default();
            // Remove quotes from JSON string if any
            let clean = serialized.trim_matches('"');
            let _ = s.set_item("app-theme", clean);
        }
    });

    provide_context(ThemeContext { theme });
}

pub fn use_theme() -> ThemeContext {
    use_context::<ThemeContext>().expect("ThemeContext not provided")
}

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Available themes for the application.
///
/// Curated selection of professional dark themes with distinctive accents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Theme {
    // Original themes
    #[default]
    Industrial,    // Default cyan industrial theme
    Indigo,        // Deep navy with electric indigo glow
    Coffee,        // Warm carbon with brown undertones
    Amber,         // Warm industrial with orange accent
    ZincBlue,      // Zinc neutral + blue accent (Vercel-inspired)
    SlateTeal,     // Slate neutral + teal accent
    StoneOrange,   // Stone warm neutral + orange accent
    NeutralGreen,  // Pure neutral + green accent
    // Tweakcn themes
    AmberMinimal,
    AmethystHaze,
    Caffeine,
    Catppuccin,
    Claude,
    Claymorphism,
    CleanSlate,
    CosmicNight,
    Darkmatter,
    Doom64,
    Graphite,
    Mono,
    OceanBreeze,
    SageGarden,
    SoftPop,
    SolarDusk,
    Supabase,
    Twitter,
    Vercel,
    VioletBloom,
}

impl Theme {
    pub fn class_name(&self) -> &'static str {
        match self {
            // Original themes
            Theme::Industrial => "", // Default uses :root variables
            Theme::Indigo => "theme-indigo",
            Theme::Coffee => "theme-coffee",
            Theme::Amber => "theme-amber",
            Theme::ZincBlue => "theme-zinc-blue",
            Theme::SlateTeal => "theme-slate-teal",
            Theme::StoneOrange => "theme-stone-orange",
            Theme::NeutralGreen => "theme-neutral-green",
            // Tweakcn themes
            Theme::AmberMinimal => "theme-amber-minimal",
            Theme::AmethystHaze => "theme-amethyst-haze",
            Theme::Caffeine => "theme-caffeine",
            Theme::Catppuccin => "theme-catppuccin",
            Theme::Claude => "theme-claude",
            Theme::Claymorphism => "theme-claymorphism",
            Theme::CleanSlate => "theme-clean-slate",
            Theme::CosmicNight => "theme-cosmic-night",
            Theme::Darkmatter => "theme-darkmatter",
            Theme::Doom64 => "theme-doom-64",
            Theme::Graphite => "theme-graphite",
            Theme::Mono => "theme-mono",
            Theme::OceanBreeze => "theme-ocean-breeze",
            Theme::SageGarden => "theme-sage-garden",
            Theme::SoftPop => "theme-soft-pop",
            Theme::SolarDusk => "theme-solar-dusk",
            Theme::Supabase => "theme-supabase",
            Theme::Twitter => "theme-twitter",
            Theme::Vercel => "theme-vercel",
            Theme::VioletBloom => "theme-violet-bloom",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            // Original themes
            Theme::Industrial => "Industrial Dark",
            Theme::Indigo => "Indigo Night",
            Theme::Coffee => "Coffee Bean",
            Theme::Amber => "Amber Glow",
            Theme::ZincBlue => "Zinc Blue",
            Theme::SlateTeal => "Slate Teal",
            Theme::StoneOrange => "Stone Orange",
            Theme::NeutralGreen => "Neutral Green",
            // Tweakcn themes
            Theme::AmberMinimal => "Amber Minimal",
            Theme::AmethystHaze => "Amethyst Haze",
            Theme::Caffeine => "Caffeine",
            Theme::Catppuccin => "Catppuccin",
            Theme::Claude => "Claude",
            Theme::Claymorphism => "Claymorphism",
            Theme::CleanSlate => "Clean Slate",
            Theme::CosmicNight => "Cosmic Night",
            Theme::Darkmatter => "Dark Matter",
            Theme::Doom64 => "Doom 64",
            Theme::Graphite => "Graphite",
            Theme::Mono => "Mono",
            Theme::OceanBreeze => "Ocean Breeze",
            Theme::SageGarden => "Sage Garden",
            Theme::SoftPop => "Soft Pop",
            Theme::SolarDusk => "Solar Dusk",
            Theme::Supabase => "Supabase",
            Theme::Twitter => "Twitter",
            Theme::Vercel => "Vercel",
            Theme::VioletBloom => "Violet Bloom",
        }
    }

    pub fn preview_color(&self) -> &'static str {
        match self {
            // Original themes
            Theme::Industrial => "#00d9ff",  // Cyan
            Theme::Indigo => "#818cf8",      // Electric indigo
            Theme::Coffee => "#d97706",      // Warm brown
            Theme::Amber => "#fbbf24",       // Warm orange
            Theme::ZincBlue => "#3b82f6",    // Blue-500
            Theme::SlateTeal => "#14b8a6",   // Teal-500
            Theme::StoneOrange => "#f97316", // Orange-500
            Theme::NeutralGreen => "#22c55e",// Green-500
            // Tweakcn themes
            Theme::AmberMinimal => "#f59d0a",
            Theme::AmethystHaze => "#a994c9",
            Theme::Caffeine => "#ffdfc1",
            Theme::Catppuccin => "#caa5f6",
            Theme::Claude => "#d87757",
            Theme::Claymorphism => "#808bf7",
            Theme::CleanSlate => "#808bf7",
            Theme::CosmicNight => "#a48ffe",
            Theme::Darkmatter => "#e78952",
            Theme::Doom64 => "#e53834",
            Theme::Graphite => "#b3b3b3",
            Theme::Mono => "#8d8d8d",
            Theme::OceanBreeze => "#33d298",
            Theme::SageGarden => "#7c9082",
            Theme::SoftPop => "#808bf7",
            Theme::SolarDusk => "#f97316",
            Theme::Supabase => "#006138",
            Theme::Twitter => "#1c9cef",
            Theme::Vercel => "#ffffff",
            Theme::VioletBloom => "#8c5cfe",
        }
    }

    pub fn all() -> &'static [Theme] {
        &[
            // Original themes
            Theme::Industrial,
            Theme::Indigo,
            Theme::Coffee,
            Theme::Amber,
            Theme::ZincBlue,
            Theme::SlateTeal,
            Theme::StoneOrange,
            Theme::NeutralGreen,
            // Tweakcn themes
            Theme::AmberMinimal,
            Theme::AmethystHaze,
            Theme::Caffeine,
            Theme::Catppuccin,
            Theme::Claude,
            Theme::Claymorphism,
            Theme::CleanSlate,
            Theme::CosmicNight,
            Theme::Darkmatter,
            Theme::Doom64,
            Theme::Graphite,
            Theme::Mono,
            Theme::OceanBreeze,
            Theme::SageGarden,
            Theme::SoftPop,
            Theme::SolarDusk,
            Theme::Supabase,
            Theme::Twitter,
            Theme::Vercel,
            Theme::VioletBloom,
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
    let stored_value = storage.as_ref()
        .and_then(|s| s.get_item("app-theme").ok().flatten());

    log::debug!("[Theme] Raw localStorage value: {:?}", stored_value);

    let initial_theme: Theme = stored_value
        .as_ref()
        .and_then(|t| {
            let json_str = format!("\"{}\"", t);
            log::debug!("[Theme] Attempting to parse: {}", json_str);
            let result = serde_json::from_str::<Theme>(&json_str);
            log::debug!("[Theme] Parse result: {:?}", result);
            result.ok()
        })
        .unwrap_or_default();

    log::debug!("[Theme] Initial theme: {:?}", initial_theme);

    let theme = RwSignal::new(initial_theme);

    // Track the previous theme to avoid unnecessary localStorage writes
    let prev_theme = RwSignal::new(initial_theme);

    // Apply initial theme to DOM immediately (not in effect)
    apply_theme_to_dom(&initial_theme);

    // Effect to persist theme and update body class on CHANGES only
    Effect::new(move |_| {
        let current_theme = theme.get();
        let previous = prev_theme.get_untracked();

        // Only act if theme actually changed
        if current_theme != previous {
            log::debug!("[Theme] Theme changed from {:?} to {:?}", previous, current_theme);

            // Update DOM
            apply_theme_to_dom(&current_theme);

            // Persist to localStorage
            if let Some(s) = &storage {
                let serialized = serde_json::to_string(&current_theme).unwrap_or_default();
                let clean = serialized.trim_matches('"');
                log::debug!("[Theme] Saving to localStorage: {}", clean);
                let _ = s.set_item("app-theme", clean);
            }

            // Update previous
            prev_theme.set(current_theme);
        }
    });

    provide_context(ThemeContext { theme });
}

fn apply_theme_to_dom(theme: &Theme) {
    let class = theme.class_name();

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
        log::debug!("[Theme] Applied class '{}' to body", class);
    }
}

pub fn use_theme() -> ThemeContext {
    use_context::<ThemeContext>().expect("ThemeContext not provided")
}

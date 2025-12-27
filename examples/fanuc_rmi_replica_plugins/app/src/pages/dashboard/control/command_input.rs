//! Command input section with recent commands and composer button.
//!
//! NOTE: This component is now consolidated into QuickCommandsPanel.
//! This file is kept for backwards compatibility but the component is deprecated.

use leptos::prelude::*;

/// Command input section - DEPRECATED: Use QuickCommandsPanel instead.
/// This component is now consolidated into the QuickCommandsPanel for a more
/// compact, responsive layout.
#[component]
#[allow(dead_code)]
pub fn CommandInputSection() -> impl IntoView {
    // This component is deprecated - return empty view
    // All functionality is now in QuickCommandsPanel
    view! {
        // Deprecated - functionality moved to QuickCommandsPanel
    }
}

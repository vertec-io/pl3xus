//! Ready-to-use components for common patterns.
//!
//! This module provides high-level components that wrap the hooks in `hooks.rs`
//! to provide complete, ready-to-use UI elements.

use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

use leptos::prelude::*;

use crate::hooks::use_sync_field_editor;
use crate::traits::SyncComponent;

/// A ready-to-use editable input component with Enter-to-apply, blur-to-revert UX.
///
/// This component wraps `use_sync_field_editor` and provides a complete input field
/// with all the necessary event handlers. It implements the NodeRef + Effect + focus
/// tracking pattern to achieve focus retention through server updates.
///
/// # Type Parameters
///
/// - `T`: The component type (must implement `SyncComponent`)
/// - `F`: The field type (must implement `Display + FromStr`)
///
/// # Props
///
/// - `entity_id`: The entity to edit
/// - `field_accessor`: A function that extracts the field value from the component
/// - `field_mutator`: A function that creates a new component with the field updated
/// - `input_type`: The HTML input type (default: "text")
/// - `class`: CSS class for the input element (optional)
/// - `placeholder`: Placeholder text (optional)
///
/// # Example
///
/// ```rust,ignore
/// use pl3xus_client::{SyncFieldInput, SyncComponent};
/// use leptos::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Clone, Default, Serialize, Deserialize)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// // SyncComponent is automatically implemented!
///
/// #[component]
/// fn PositionEditor(entity_id: u64) -> impl IntoView {
///     view! {
///         <div class="position-editor">
///             <label>
///                 "X: "
///                 <SyncFieldInput
///                     entity_id=entity_id
///                     field_accessor=|pos: &Position| pos.x
///                     field_mutator=|pos: &Position, new_x: f32| Position { x: new_x, y: pos.y }
///                     input_type="number"
///                     class="number-input"
///                 />
///             </label>
///             <label>
///                 "Y: "
///                 <SyncFieldInput
///                     entity_id=entity_id
///                     field_accessor=|pos: &Position| pos.y
///                     field_mutator=|pos: &Position, new_y: f32| Position { x: pos.x, y: new_y }
///                     input_type="number"
///                     class="number-input"
///                 />
///             </label>
///         </div>
///     }
/// }
/// ```
#[component]
pub fn SyncFieldInput<T, F, A, M>(
    /// The entity to edit
    entity_id: u64,
    /// Function that extracts the field value from the component
    field_accessor: A,
    /// Function that creates a new component with the field updated
    field_mutator: M,
    /// HTML input type (default: "text")
    #[prop(default = "text")]
    input_type: &'static str,
    /// CSS class for the input element
    #[prop(optional)]
    class: Option<&'static str>,
    /// Placeholder text
    #[prop(optional)]
    placeholder: Option<&'static str>,
    /// Phantom data to mark type parameters as used
    #[prop(optional)]
    _phantom: PhantomData<(T, F)>,
) -> impl IntoView
where
    T: SyncComponent + Clone + Default + 'static,
    F: Display + FromStr + Clone + PartialEq + 'static,
    A: Fn(&T) -> F + Clone + 'static,
    M: Fn(&T, F) -> T + Clone + 'static,
{
    let (input_ref, is_focused, initial_value, on_keydown, on_blur_handler) =
        use_sync_field_editor(entity_id, field_accessor, field_mutator);

    view! {
        <input
            node_ref=input_ref
            type=input_type
            value=initial_value
            class=class.unwrap_or("")
            placeholder=placeholder.unwrap_or("")
            on:focus=move |_| is_focused.set(true)
            on:blur=move |_| {
                is_focused.set(false);
                on_blur_handler();
            }
            on:keydown=on_keydown
        />
    }
}


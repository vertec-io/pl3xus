//! Robot Creation Wizard - Production-grade multi-step wizard for creating robots with configurations.
//!
//! This wizard guides users through creating a robot connection with at least one configuration.
//! It ensures data integrity and provides excellent UX with validation, progress tracking, and exit warnings.

use leptos::prelude::*;

use pl3xus_client::use_mutation;
use fanuc_replica_types::*;

/// Wizard step enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    ConnectionDetails,
    MotionDefaults,
    JogDefaults,
    DefaultConfiguration,
    AdditionalConfigurations,
}

impl WizardStep {
    fn step_number(&self) -> usize {
        match self {
            WizardStep::ConnectionDetails => 1,
            WizardStep::MotionDefaults => 2,
            WizardStep::JogDefaults => 3,
            WizardStep::DefaultConfiguration => 4,
            WizardStep::AdditionalConfigurations => 5,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            WizardStep::ConnectionDetails => "Connection Details",
            WizardStep::MotionDefaults => "Motion Defaults",
            WizardStep::JogDefaults => "Jog Defaults",
            WizardStep::DefaultConfiguration => "Default Configuration",
            WizardStep::AdditionalConfigurations => "Additional Configurations",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            WizardStep::ConnectionDetails => "Enter robot name, IP address, and connection details",
            WizardStep::MotionDefaults => "Set default motion parameters for this robot",
            WizardStep::JogDefaults => "Configure jogging speeds and step sizes",
            WizardStep::DefaultConfiguration => "Create the default configuration (required)",
            WizardStep::AdditionalConfigurations => "Add more configurations (optional)",
        }
    }
}

/// Robot Creation Wizard Component
#[component]
pub fn RobotCreationWizard<F1, F2>(
    on_close: F1,
    on_created: F2,
) -> impl IntoView
where
    F1: Fn() + Clone + Send + Sync + 'static,
    F2: Fn(i64) + Clone + Send + Sync + 'static,
{
    // Wizard state
    let (current_step, set_current_step) = signal(WizardStep::ConnectionDetails);
    let (show_exit_warning, set_show_exit_warning) = signal(false);
    let (validation_error, set_validation_error) = signal::<Option<String>>(None);
    let (is_submitting, set_is_submitting) = signal(false);

    // Step 1: Connection Details
    let (robot_name, set_robot_name) = signal(String::new());
    let (robot_description, set_robot_description) = signal(String::new());
    let (robot_ip, set_robot_ip) = signal("127.0.0.1".to_string());
    let (robot_port, set_robot_port) = signal("16001".to_string());

    // Step 2: Motion Defaults
    let (default_speed, set_default_speed) = signal("100.0".to_string());
    let (default_speed_type, set_default_speed_type) = signal("mmSec".to_string());
    let (default_term_type, set_default_term_type) = signal("CNT".to_string());
    let (default_w, set_default_w) = signal("0.0".to_string());
    let (default_p, set_default_p) = signal("0.0".to_string());
    let (default_r, set_default_r) = signal("0.0".to_string());

    // Step 3: Jog Defaults
    let (cartesian_jog_speed, set_cartesian_jog_speed) = signal("10.0".to_string());
    let (cartesian_jog_step, set_cartesian_jog_step) = signal("1.0".to_string());
    let (joint_jog_speed, set_joint_jog_speed) = signal("10.0".to_string()); // °/s
    let (joint_jog_step, set_joint_jog_step) = signal("1.0".to_string()); // degrees
    let (rotation_jog_speed, set_rotation_jog_speed) = signal("5.0".to_string());
    let (rotation_jog_step, set_rotation_jog_step) = signal("1.0".to_string());

    // Step 4: Default Configuration
    let (config_name, set_config_name) = signal("Default".to_string());
    let (config_uframe, set_config_uframe) = signal("0".to_string());
    let (config_utool, set_config_utool) = signal("1".to_string());
    let (config_front, set_config_front) = signal("1".to_string());
    let (config_up, set_config_up) = signal("1".to_string());
    let (config_left, set_config_left) = signal("0".to_string());
    let (config_flip, set_config_flip) = signal("0".to_string());
    let (config_turn4, set_config_turn4) = signal("0".to_string());
    let (config_turn5, set_config_turn5) = signal("0".to_string());
    let (config_turn6, set_config_turn6) = signal("0".to_string());

    // CreateRobotConnection mutation
    let on_created_for_handler = on_created.clone();
    let create_robot = use_mutation::<CreateRobotConnection>(move |result| {
        set_is_submitting.set(false);
        match result {
            Ok(r) if r.success => on_created_for_handler(r.robot_id),
            Ok(r) => set_validation_error.set(r.error.clone()),
            Err(e) => set_validation_error.set(Some(e.to_string())),
        }
    });

    // Validation helper
    let validate_current_step = move || -> Result<(), String> {
        match current_step.get() {
            WizardStep::ConnectionDetails => {
                if robot_name.get().trim().is_empty() {
                    return Err("Robot name is required".to_string());
                }
                if robot_ip.get().trim().is_empty() {
                    return Err("IP address is required".to_string());
                }
                robot_port.get().parse::<u32>()
                    .map_err(|_| "Invalid port number".to_string())?;
                Ok(())
            }
            WizardStep::MotionDefaults => {
                let speed = default_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid speed value".to_string())?;
                if !(0.0..=100.0).contains(&speed) {
                    return Err("Speed must be between 0 and 100".to_string());
                }
                default_w.get().parse::<f64>()
                    .map_err(|_| "Invalid W value".to_string())?;
                default_p.get().parse::<f64>()
                    .map_err(|_| "Invalid P value".to_string())?;
                default_r.get().parse::<f64>()
                    .map_err(|_| "Invalid R value".to_string())?;
                Ok(())
            }
            WizardStep::JogDefaults => {
                cartesian_jog_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid cartesian jog speed".to_string())?;
                cartesian_jog_step.get().parse::<f64>()
                    .map_err(|_| "Invalid cartesian jog step".to_string())?;
                let joint_speed = joint_jog_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid joint jog speed".to_string())?;
                if !(0.0..=100.0).contains(&joint_speed) {
                    return Err("Joint jog speed must be between 0 and 100".to_string());
                }
                joint_jog_step.get().parse::<f64>()
                    .map_err(|_| "Invalid joint jog step".to_string())?;
                rotation_jog_speed.get().parse::<f64>()
                    .map_err(|_| "Invalid rotation jog speed".to_string())?;
                rotation_jog_step.get().parse::<f64>()
                    .map_err(|_| "Invalid rotation jog step".to_string())?;
                Ok(())
            }
            WizardStep::DefaultConfiguration => {
                if config_name.get().trim().is_empty() {
                    return Err("Configuration name is required".to_string());
                }
                let uframe = config_uframe.get().parse::<i32>()
                    .map_err(|_| "Invalid UFrame number".to_string())?;
                if !(0..=9).contains(&uframe) {
                    return Err("UFrame must be between 0 and 9".to_string());
                }
                let utool = config_utool.get().parse::<i32>()
                    .map_err(|_| "Invalid UTool number".to_string())?;
                if !(1..=10).contains(&utool) {
                    return Err("UTool must be between 1 and 10 (Tool 0 is invalid)".to_string());
                }
                config_turn4.get().parse::<i32>()
                    .map_err(|_| "Invalid Turn 4 value".to_string())?;
                config_turn5.get().parse::<i32>()
                    .map_err(|_| "Invalid Turn 5 value".to_string())?;
                config_turn6.get().parse::<i32>()
                    .map_err(|_| "Invalid Turn 6 value".to_string())?;
                Ok(())
            }
            WizardStep::AdditionalConfigurations => Ok(()),
        }
    };

    // Submit handler
    let submit_robot = {
        move || {
            if is_submitting.get() { return; }
            set_validation_error.set(None);

            // Validate all steps
            for step in [WizardStep::ConnectionDetails, WizardStep::MotionDefaults,
                         WizardStep::JogDefaults, WizardStep::DefaultConfiguration] {
                set_current_step.set(step);
                if let Err(e) = validate_current_step() {
                    set_validation_error.set(Some(e));
                    return;
                }
            }
            set_current_step.set(WizardStep::DefaultConfiguration);
            set_is_submitting.set(true);

            create_robot.send(CreateRobotConnection {
                name: robot_name.get(),
                description: if robot_description.get().is_empty() { None } else { Some(robot_description.get()) },
                ip_address: robot_ip.get(),
                port: robot_port.get().parse().unwrap(),
                default_speed: default_speed.get().parse().unwrap(),
                default_speed_type: default_speed_type.get(),
                default_term_type: default_term_type.get(),
                default_w: default_w.get().parse().unwrap(),
                default_p: default_p.get().parse().unwrap(),
                default_r: default_r.get().parse().unwrap(),
                default_cartesian_jog_speed: cartesian_jog_speed.get().parse().unwrap(),
                default_cartesian_jog_step: cartesian_jog_step.get().parse().unwrap(),
                default_joint_jog_speed: joint_jog_speed.get().parse().unwrap(),
                default_joint_jog_step: joint_jog_step.get().parse().unwrap(),
                default_rotation_jog_speed: rotation_jog_speed.get().parse().unwrap(),
                default_rotation_jog_step: rotation_jog_step.get().parse().unwrap(),
                configuration: NewRobotConfiguration {
                    name: config_name.get(),
                    is_default: true,
                    u_frame_number: config_uframe.get().parse().unwrap(),
                    u_tool_number: config_utool.get().parse().unwrap(),
                    front: config_front.get().parse().unwrap(),
                    up: config_up.get().parse().unwrap(),
                    left: config_left.get().parse().unwrap(),
                    flip: config_flip.get().parse().unwrap(),
                    turn4: config_turn4.get().parse().unwrap(),
                    turn5: config_turn5.get().parse().unwrap(),
                    turn6: config_turn6.get().parse().unwrap(),
                },
            });
        }
    };

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
            <div class="bg-background border border-border/8 rounded-lg shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col">
                // Header
                <WizardHeader current_step=current_step />

                // Content (scrollable)
                <div class="flex-1 overflow-y-auto p-6">
                    <Show when=move || current_step.get() == WizardStep::ConnectionDetails>
                        <ConnectionDetailsStep
                            robot_name=robot_name set_robot_name=set_robot_name
                            robot_description=robot_description set_robot_description=set_robot_description
                            robot_ip=robot_ip set_robot_ip=set_robot_ip
                            robot_port=robot_port set_robot_port=set_robot_port
                        />
                    </Show>
                    <Show when=move || current_step.get() == WizardStep::MotionDefaults>
                        <MotionDefaultsStep
                            default_speed=default_speed set_default_speed=set_default_speed
                            default_speed_type=default_speed_type set_default_speed_type=set_default_speed_type
                            default_term_type=default_term_type set_default_term_type=set_default_term_type
                            default_w=default_w set_default_w=set_default_w
                            default_p=default_p set_default_p=set_default_p
                            default_r=default_r set_default_r=set_default_r
                        />
                    </Show>
                    <Show when=move || current_step.get() == WizardStep::JogDefaults>
                        <JogDefaultsStep
                            cartesian_jog_speed=cartesian_jog_speed set_cartesian_jog_speed=set_cartesian_jog_speed
                            cartesian_jog_step=cartesian_jog_step set_cartesian_jog_step=set_cartesian_jog_step
                            joint_jog_speed=joint_jog_speed set_joint_jog_speed=set_joint_jog_speed
                            joint_jog_step=joint_jog_step set_joint_jog_step=set_joint_jog_step
                            rotation_jog_speed=rotation_jog_speed set_rotation_jog_speed=set_rotation_jog_speed
                            rotation_jog_step=rotation_jog_step set_rotation_jog_step=set_rotation_jog_step
                        />
                    </Show>
                    <Show when=move || current_step.get() == WizardStep::DefaultConfiguration>
                        <DefaultConfigurationStep
                            config_name=config_name set_config_name=set_config_name
                            u_frame_number=config_uframe set_u_frame_number=set_config_uframe
                            u_tool_number=config_utool set_u_tool_number=set_config_utool
                            front=config_front set_front=set_config_front
                            up=config_up set_up=set_config_up
                            left=config_left set_left=set_config_left
                            flip=config_flip set_flip=set_config_flip
                            turn4=config_turn4 set_turn4=set_config_turn4
                            turn5=config_turn5 set_turn5=set_config_turn5
                            turn6=config_turn6 set_turn6=set_config_turn6
                        />
                    </Show>

                    <Show when=move || current_step.get() == WizardStep::AdditionalConfigurations>
                        <div class="text-center py-8">
                            <p class="text-muted-foreground text-sm">"Additional configurations feature coming soon"</p>
                            <p class="text-muted-foreground text-xs mt-2">"You can add more configurations after creating the robot"</p>
                        </div>
                    </Show>
                </div>

                // Footer with navigation
                <WizardFooter
                    current_step=current_step
                    set_current_step=set_current_step
                    set_show_exit_warning=set_show_exit_warning
                    validation_error=validation_error
                    set_validation_error=set_validation_error
                    validate_current_step=validate_current_step
                    submit_robot=submit_robot.clone()
                    is_submitting=is_submitting
                />
            </div>

            // Exit Warning Modal
            <Show when=move || show_exit_warning.get()>
                <ExitWarningModal
                    on_cancel=move || set_show_exit_warning.set(false)
                    on_confirm={
                        let on_close = on_close.clone();
                        move || {
                            set_show_exit_warning.set(false);
                            on_close();
                        }
                    }
                />
            </Show>
        </div>
    }
}

// ============================================================================
// Wizard Header with progress indicator
// ============================================================================

#[component]
fn WizardHeader(current_step: ReadSignal<WizardStep>) -> impl IntoView {
    let steps = vec![
        WizardStep::ConnectionDetails,
        WizardStep::MotionDefaults,
        WizardStep::JogDefaults,
        WizardStep::DefaultConfiguration,
        WizardStep::AdditionalConfigurations,
    ];

    view! {
        <div class="border-b border-border/8 p-6">
            <h2 class="text-lg font-semibold text-white mb-4">"Create New Robot"</h2>

            // Progress indicator
            <div class="flex items-center gap-2">
                <For
                    each=move || steps.clone().into_iter().enumerate()
                    key=|(idx, _)| *idx
                    children=move |(idx, step)| {
                        let is_current = move || current_step.get() == step;
                        let is_completed = move || current_step.get().step_number() > step.step_number();

                        view! {
                            <div class=move || format!(
                                "flex items-center justify-center w-8 h-8 rounded-full text-xs font-semibold transition-colors {}",
                                if is_current() {
                                    "bg-primary text-black"
                                } else if is_completed() {
                                    "bg-success text-black"
                                } else {
                                    "bg-popover text-muted-foreground border border-border/8"
                                }
                            )>
                                {move || if is_completed() { "✓".to_string() } else { (idx + 1).to_string() }}
                            </div>

                            <Show when=move || idx < 4>
                                <div class=move || format!(
                                    "h-0.5 w-12 transition-colors {}",
                                    if is_completed() { "bg-success" } else { "bg-border" }
                                )></div>
                            </Show>
                        }
                    }
                />
            </div>

            // Current step title and description
            <div class="mt-4">
                <h3 class="text-sm font-semibold text-white">{move || current_step.get().title()}</h3>
                <p class="text-xs text-muted-foreground mt-1">{move || current_step.get().description()}</p>
            </div>
        </div>
    }
}

// ============================================================================
// Wizard Footer with navigation buttons
// ============================================================================

#[component]
fn WizardFooter<V, S>(
    current_step: ReadSignal<WizardStep>,
    set_current_step: WriteSignal<WizardStep>,
    set_show_exit_warning: WriteSignal<bool>,
    validation_error: ReadSignal<Option<String>>,
    set_validation_error: WriteSignal<Option<String>>,
    validate_current_step: V,
    submit_robot: S,
    is_submitting: ReadSignal<bool>,
) -> impl IntoView
where
    V: Fn() -> Result<(), String> + Clone + Send + Sync + 'static,
    S: Fn() + Clone + Send + Sync + 'static,
{
    let can_go_back = move || current_step.get() != WizardStep::ConnectionDetails;
    let can_go_next = move || current_step.get() != WizardStep::AdditionalConfigurations;
    let is_last_required_step = move || current_step.get() == WizardStep::DefaultConfiguration;

    let go_back = move |_| {
        set_validation_error.set(None);
        let new_step = match current_step.get() {
            WizardStep::MotionDefaults => WizardStep::ConnectionDetails,
            WizardStep::JogDefaults => WizardStep::MotionDefaults,
            WizardStep::DefaultConfiguration => WizardStep::JogDefaults,
            WizardStep::AdditionalConfigurations => WizardStep::DefaultConfiguration,
            _ => current_step.get(),
        };
        set_current_step.set(new_step);
    };

    let validate_clone = validate_current_step.clone();
    let go_next = move |_| {
        set_validation_error.set(None);
        if let Err(e) = validate_clone() {
            set_validation_error.set(Some(e));
            return;
        }
        let new_step = match current_step.get() {
            WizardStep::ConnectionDetails => WizardStep::MotionDefaults,
            WizardStep::MotionDefaults => WizardStep::JogDefaults,
            WizardStep::JogDefaults => WizardStep::DefaultConfiguration,
            WizardStep::DefaultConfiguration => WizardStep::AdditionalConfigurations,
            _ => current_step.get(),
        };
        set_current_step.set(new_step);
    };

    let submit_clone = submit_robot.clone();

    view! {
        <div class="border-t border-border/8 p-6">
            {move || validation_error.get().map(|err| view! {
                <div class="mb-4 p-3 bg-destructive/10 border border-destructive rounded text-sm text-destructive">
                    {err}
                </div>
            })}

            <div class="flex items-center justify-between">
                <button
                    type="button"
                    class="px-4 py-2 text-sm text-muted-foreground hover:text-white transition-colors"
                    on:click=move |_| set_show_exit_warning.set(true)
                >
                    "Cancel"
                </button>

                <div class="flex gap-3">
                    <Show when=can_go_back>
                        <button
                            type="button"
                            class="px-4 py-2 bg-popover border border-border/8 rounded text-sm text-white hover:bg-secondary transition-colors"
                            on:click=go_back
                        >
                            "← Back"
                        </button>
                    </Show>

                    <Show when=can_go_next>
                        {
                            let go_next = go_next.clone();
                            move || view! {
                                <button
                                    type="button"
                                    class="px-4 py-2 bg-primary rounded text-sm text-black font-semibold hover:bg-primary transition-colors"
                                    on:click=go_next.clone()
                                >
                                    "Next →"
                                </button>
                            }
                        }
                    </Show>

                    <Show when=is_last_required_step>
                        {
                            let submit_clone = submit_clone.clone();
                            move || view! {
                                <button
                                    type="button"
                                    class=move || format!(
                                        "px-4 py-2 bg-success rounded text-sm text-black font-semibold transition-colors {}",
                                        if is_submitting.get() { "opacity-50 cursor-not-allowed" } else { "hover:bg-success" }
                                    )
                                    disabled=move || is_submitting.get()
                                    on:click={
                                        let submit_clone = submit_clone.clone();
                                        move |_| submit_clone()
                                    }
                                >
                                    {move || if is_submitting.get() { "Creating..." } else { "Create Robot" }}
                                </button>
                            }
                        }
                    </Show>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Exit Warning Modal
// ============================================================================

#[component]
fn ExitWarningModal<F1, F2>(on_cancel: F1, on_confirm: F2) -> impl IntoView
where
    F1: Fn() + Clone + Send + Sync + 'static,
    F2: Fn() + Clone + Send + Sync + 'static,
{
    let cancel = on_cancel.clone();
    let confirm = on_confirm.clone();

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-[60]">
            <div class="bg-background border border-border/8 rounded-lg shadow-2xl w-full max-w-md p-6">
                <div class="flex items-start gap-3 mb-4">
                    <div class="w-10 h-10 rounded-full bg-destructive/10 border border-destructive flex items-center justify-center flex-shrink-0">
                        <svg class="w-5 h-5 text-destructive" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                    </div>
                    <div class="flex-1">
                        <h3 class="text-base font-semibold text-white mb-1">"Discard Robot Creation?"</h3>
                        <p class="text-sm text-muted-foreground">
                            "Your robot configuration will not be saved if you exit now. Are you sure you want to cancel?"
                        </p>
                    </div>
                </div>

                <div class="flex gap-3 justify-end">
                    <button
                        type="button"
                        class="px-4 py-2 bg-popover border border-border/8 rounded text-sm text-white hover:bg-secondary transition-colors"
                        on:click=move |_| cancel()
                    >
                        "Continue Editing"
                    </button>
                    <button
                        type="button"
                        class="px-4 py-2 bg-destructive rounded text-sm text-white font-semibold hover:bg-destructive transition-colors"
                        on:click=move |_| confirm()
                    >
                        "Discard Changes"
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Step Components
// ============================================================================

#[component]
fn ConnectionDetailsStep(
    robot_name: ReadSignal<String>,
    set_robot_name: WriteSignal<String>,
    robot_description: ReadSignal<String>,
    set_robot_description: WriteSignal<String>,
    robot_ip: ReadSignal<String>,
    set_robot_ip: WriteSignal<String>,
    robot_port: ReadSignal<String>,
    set_robot_port: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div>
                <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                    "Robot Name" <span class="text-destructive">"*"</span>
                </label>
                <input
                    type="text"
                    class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                    placeholder="e.g., Production Cell 1"
                    prop:value=move || robot_name.get()
                    on:input=move |ev| set_robot_name.set(event_target_value(&ev))
                />
                <p class="text-xs text-muted-foreground mt-1">"A descriptive name for this robot"</p>
            </div>

            <div>
                <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Description (optional)"</label>
                <input
                    type="text"
                    class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                    placeholder="e.g., Main welding robot"
                    prop:value=move || robot_description.get()
                    on:input=move |ev| set_robot_description.set(event_target_value(&ev))
                />
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "IP Address" <span class="text-destructive">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                        placeholder="192.168.1.100"
                        prop:value=move || robot_ip.get()
                        on:input=move |ev| set_robot_ip.set(event_target_value(&ev))
                    />
                </div>

                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "Port" <span class="text-destructive">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                        placeholder="16001"
                        prop:value=move || robot_port.get()
                        on:input=move |ev| set_robot_port.set(event_target_value(&ev))
                    />
                </div>
            </div>
        </div>
    }
}

#[component]
fn MotionDefaultsStep(
    default_speed: ReadSignal<String>,
    set_default_speed: WriteSignal<String>,
    default_speed_type: ReadSignal<String>,
    set_default_speed_type: WriteSignal<String>,
    default_term_type: ReadSignal<String>,
    set_default_term_type: WriteSignal<String>,
    default_w: ReadSignal<String>,
    set_default_w: WriteSignal<String>,
    default_p: ReadSignal<String>,
    set_default_p: WriteSignal<String>,
    default_r: ReadSignal<String>,
    set_default_r: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "Default Speed" <span class="text-destructive">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                        prop:value=move || default_speed.get()
                        on:input=move |ev| set_default_speed.set(event_target_value(&ev))
                        placeholder="100.0"
                    />
                    <p class="text-xs text-muted-foreground mt-1">"Speed value (units depend on type)"</p>
                </div>

                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "Speed Type" <span class="text-destructive">"*"</span>
                    </label>
                    <select
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                        prop:value=move || default_speed_type.get()
                        on:change=move |ev| set_default_speed_type.set(event_target_value(&ev))
                    >
                        <option value="mmSec">"mm/sec (Linear)"</option>
                        <option value="InchMin">"0.1 inch/min"</option>
                        <option value="Time">"0.1 seconds (Time-based)"</option>
                        <option value="mSec">"milliseconds"</option>
                    </select>
                    <p class="text-xs text-muted-foreground mt-1">"How speed values are interpreted"</p>
                </div>
            </div>

            <div class="grid grid-cols-1 gap-4">
                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "Termination Type" <span class="text-destructive">"*"</span>
                    </label>
                    <select
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                        prop:value=move || default_term_type.get()
                        on:change=move |ev| set_default_term_type.set(event_target_value(&ev))
                    >
                        <option value="CNT">"CNT (Continuous)"</option>
                        <option value="FINE">"FINE (Precise)"</option>
                    </select>
                    <p class="text-xs text-muted-foreground mt-1">"Motion termination type"</p>
                </div>
            </div>

            <div class="border-t border-border/8 pt-4">
                <h4 class="text-sm font-semibold text-white mb-3">"Wrist Singularity Avoidance"</h4>
                <div class="grid grid-cols-3 gap-4">
                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"W (Wrist)"</label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || default_w.get()
                            on:input=move |ev| set_default_w.set(event_target_value(&ev))
                            placeholder="0.0"
                        />
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"P (Pitch)"</label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || default_p.get()
                            on:input=move |ev| set_default_p.set(event_target_value(&ev))
                            placeholder="0.0"
                        />
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"R (Roll)"</label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || default_r.get()
                            on:input=move |ev| set_default_r.set(event_target_value(&ev))
                            placeholder="0.0"
                        />
                    </div>
                </div>
                <p class="text-xs text-muted-foreground mt-2">"Typically 0.0 for all axes (no avoidance)"</p>
            </div>
        </div>
    }
}

#[component]
fn JogDefaultsStep(
    cartesian_jog_speed: ReadSignal<String>,
    set_cartesian_jog_speed: WriteSignal<String>,
    cartesian_jog_step: ReadSignal<String>,
    set_cartesian_jog_step: WriteSignal<String>,
    joint_jog_speed: ReadSignal<String>,
    set_joint_jog_speed: WriteSignal<String>,
    joint_jog_step: ReadSignal<String>,
    set_joint_jog_step: WriteSignal<String>,
    rotation_jog_speed: ReadSignal<String>,
    set_rotation_jog_speed: WriteSignal<String>,
    rotation_jog_step: ReadSignal<String>,
    set_rotation_jog_step: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="border border-border/8 rounded-lg p-4 bg-background">
                <h4 class="text-sm font-semibold text-white mb-3">"Cartesian Jogging"</h4>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                            "Speed (mm/s)" <span class="text-destructive">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || cartesian_jog_speed.get()
                            on:input=move |ev| set_cartesian_jog_speed.set(event_target_value(&ev))
                            placeholder="10.0"
                        />
                        <p class="text-xs text-muted-foreground mt-1">"Continuous jog speed"</p>
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                            "Step Size (mm)" <span class="text-destructive">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || cartesian_jog_step.get()
                            on:input=move |ev| set_cartesian_jog_step.set(event_target_value(&ev))
                            placeholder="1.0"
                        />
                        <p class="text-xs text-muted-foreground mt-1">"Incremental jog distance"</p>
                    </div>
                </div>
            </div>

            <div class="border border-border/8 rounded-lg p-4 bg-background">
                <h4 class="text-sm font-semibold text-white mb-3">"Joint Jogging"</h4>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                            "Speed (°/s)" <span class="text-destructive">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || joint_jog_speed.get()
                            on:input=move |ev| set_joint_jog_speed.set(event_target_value(&ev))
                            placeholder="10.0"
                        />
                        <p class="text-xs text-muted-foreground mt-1">"Degrees per second"</p>
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                            "Step Size (°)" <span class="text-destructive">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || joint_jog_step.get()
                            on:input=move |ev| set_joint_jog_step.set(event_target_value(&ev))
                            placeholder="1.0"
                        />
                        <p class="text-xs text-muted-foreground mt-1">"Degrees per step"</p>
                    </div>
                </div>
            </div>

            <div class="border border-border/8 rounded-lg p-4 bg-background">
                <h4 class="text-sm font-semibold text-white mb-3">"Rotation Jogging (W/P/R)"</h4>
                <div class="grid grid-cols-2 gap-4">
                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                            "Speed (°/s)" <span class="text-destructive">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || rotation_jog_speed.get()
                            on:input=move |ev| set_rotation_jog_speed.set(event_target_value(&ev))
                            placeholder="5.0"
                        />
                        <p class="text-xs text-muted-foreground mt-1">"Rotation jog speed"</p>
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                            "Step Size (°)" <span class="text-destructive">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || rotation_jog_step.get()
                            on:input=move |ev| set_rotation_jog_step.set(event_target_value(&ev))
                            placeholder="1.0"
                        />
                        <p class="text-xs text-muted-foreground mt-1">"Incremental rotation angle"</p>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn DefaultConfigurationStep(
    config_name: ReadSignal<String>,
    set_config_name: WriteSignal<String>,
    u_frame_number: ReadSignal<String>,
    set_u_frame_number: WriteSignal<String>,
    u_tool_number: ReadSignal<String>,
    set_u_tool_number: WriteSignal<String>,
    front: ReadSignal<String>,
    set_front: WriteSignal<String>,
    up: ReadSignal<String>,
    set_up: WriteSignal<String>,
    left: ReadSignal<String>,
    set_left: WriteSignal<String>,
    flip: ReadSignal<String>,
    set_flip: WriteSignal<String>,
    turn4: ReadSignal<String>,
    set_turn4: WriteSignal<String>,
    turn5: ReadSignal<String>,
    set_turn5: WriteSignal<String>,
    turn6: ReadSignal<String>,
    set_turn6: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-4">
            <div>
                <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                    "Configuration Name" <span class="text-destructive">"*"</span>
                </label>
                <input
                    type="text"
                    class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                    prop:value=move || config_name.get()
                    on:input=move |ev| set_config_name.set(event_target_value(&ev))
                    placeholder="Default"
                />
                <p class="text-xs text-muted-foreground mt-1">"Name for this configuration preset"</p>
            </div>

            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "User Frame" <span class="text-destructive">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                        prop:value=move || u_frame_number.get()
                        on:input=move |ev| set_u_frame_number.set(event_target_value(&ev))
                        placeholder="0"
                    />
                    <p class="text-xs text-muted-foreground mt-1">"0-9 (0 = World Frame)"</p>
                </div>

                <div>
                    <label class="block text-muted-foreground text-xs mb-1.5 font-medium">
                        "User Tool" <span class="text-destructive">"*"</span>
                    </label>
                    <input
                        type="text"
                        class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                        prop:value=move || u_tool_number.get()
                        on:input=move |ev| set_u_tool_number.set(event_target_value(&ev))
                        placeholder="1"
                    />
                    <p class="text-xs text-muted-foreground mt-1">"1-10 (Tool 0 is invalid)"</p>
                </div>
            </div>

            <div class="border-t border-border/8 pt-4">
                <h4 class="text-sm font-semibold text-white mb-3">"Arm Configuration"</h4>
                <div class="grid grid-cols-4 gap-4">
                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Front"</label>
                        <select
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                            prop:value=move || front.get()
                            on:change=move |ev| set_front.set(event_target_value(&ev))
                        >
                            <option value="1">"1 (Front)"</option>
                            <option value="0">"0 (Back)"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Up"</label>
                        <select
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                            prop:value=move || up.get()
                            on:change=move |ev| set_up.set(event_target_value(&ev))
                        >
                            <option value="1">"1 (Up)"</option>
                            <option value="0">"0 (Down)"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Left"</label>
                        <select
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                            prop:value=move || left.get()
                            on:change=move |ev| set_left.set(event_target_value(&ev))
                        >
                            <option value="0">"0 (Right)"</option>
                            <option value="1">"1 (Left)"</option>
                        </select>
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Flip"</label>
                        <select
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors"
                            prop:value=move || flip.get()
                            on:change=move |ev| set_flip.set(event_target_value(&ev))
                        >
                            <option value="0">"0 (No Flip)"</option>
                            <option value="1">"1 (Flip)"</option>
                        </select>
                    </div>
                </div>
            </div>

            <div class="border-t border-border/8 pt-4">
                <h4 class="text-sm font-semibold text-white mb-3">"Turn Numbers"</h4>
                <div class="grid grid-cols-3 gap-4">
                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Turn 4"</label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || turn4.get()
                            on:input=move |ev| set_turn4.set(event_target_value(&ev))
                            placeholder="0"
                        />
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Turn 5"</label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || turn5.get()
                            on:input=move |ev| set_turn5.set(event_target_value(&ev))
                            placeholder="0"
                        />
                    </div>

                    <div>
                        <label class="block text-muted-foreground text-xs mb-1.5 font-medium">"Turn 6"</label>
                        <input
                            type="text"
                            class="w-full bg-card border border-border/8 rounded px-3 py-2 text-sm text-white focus:border-primary focus:outline-none transition-colors font-mono"
                            prop:value=move || turn6.get()
                            on:input=move |ev| set_turn6.set(event_target_value(&ev))
                            placeholder="0"
                        />
                    </div>
                </div>
                <p class="text-xs text-muted-foreground mt-2">"Wrist turn numbers for joint configuration"</p>
            </div>
        </div>
    }
}

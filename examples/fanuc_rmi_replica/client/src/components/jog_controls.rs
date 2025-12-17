//! Jog controls component for manual robot movement.

use leptos::prelude::*;

use pl3xus_client::use_sync_context;
use fanuc_replica_types::*;

/// Jog controls for robot manual movement.
#[component]
pub fn JogControls() -> impl IntoView {
    let ctx = use_sync_context();
    let (speed, set_speed) = signal(50.0f32);
    let (step, set_step) = signal(10.0f32);

    let jog = move |axis: JogAxis, direction: JogDirection| {
        ctx.send(JogCommand {
            axis,
            direction,
            distance: step.get(),
            speed: speed.get(),
        });
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide">"Jog Control"</h2>
            
            // Settings
            <div class="grid grid-cols-2 gap-2 mb-2">
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Speed"</div>
                    <input type="range" min="1" max="100" class="w-full accent-[#00d9ff] h-1"
                        prop:value=speed
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() { set_speed.set(v); }
                        }
                    />
                    <div class="text-[9px] text-[#00d9ff] text-right font-mono">{move || speed.get()}</div>
                </div>
                <div class="bg-[#111111] rounded p-1.5">
                    <div class="text-[8px] text-[#666666] mb-1">"Step"</div>
                    <input type="range" min="1" max="50" class="w-full accent-[#00d9ff] h-1"
                        prop:value=step
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<f32>() { set_step.set(v); }
                        }
                    />
                    <div class="text-[9px] text-[#00d9ff] text-right font-mono">{move || step.get()}</div>
                </div>
            </div>

            // Directional Buttons
            <div class="grid grid-cols-3 gap-1">
                <div></div>
                <JogButton label="Y+" jog=jog.clone() axis=JogAxis::Y direction=JogDirection::Positive />
                <div></div>
                <JogButton label="X-" jog=jog.clone() axis=JogAxis::X direction=JogDirection::Negative />
                <JogButton label="Z+" jog=jog.clone() axis=JogAxis::Z direction=JogDirection::Positive />
                <JogButton label="X+" jog=jog.clone() axis=JogAxis::X direction=JogDirection::Positive />
                <div></div>
                <JogButton label="Y-" jog=jog.clone() axis=JogAxis::Y direction=JogDirection::Negative />
                <JogButton label="Z-" jog=jog.clone() axis=JogAxis::Z direction=JogDirection::Negative />
            </div>
        </div>
    }
}

#[component]
fn JogButton<F>(label: &'static str, jog: F, axis: JogAxis, direction: JogDirection) -> impl IntoView 
where F: Fn(JogAxis, JogDirection) + Clone + 'static {
    let do_jog = {
        let jog = jog.clone();
        move |_| jog(axis, direction)
    };

    view! {
        <button 
            class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center text-[10px]"
            on:click=do_jog
        >
            {label}
        </button>
    }
}

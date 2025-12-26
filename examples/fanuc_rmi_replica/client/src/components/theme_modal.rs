use leptos::prelude::*;
use crate::theme::{Theme, use_theme};

#[component]
pub fn ThemeModal(
    #[prop(into)] show: RwSignal<bool>,
) -> impl IntoView {
    let theme_ctx = use_theme();
    let active_theme = theme_ctx.theme;

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-[100]"
                 on:click=move |e| {
                     if e.target() == e.current_target() {
                         show.set(false);
                     }
                 }
            >
                <div class="bg-card border border-border/10 rounded-theme shadow-theme backdrop-blur-theme w-[650px] max-h-[85vh] flex flex-col overflow-hidden animate-in fade-in zoom-in duration-200">
                    // Header
                    <div class="p-4 border-b border-border/10 flex items-center justify-between bg-muted/30">
                        <h2 class="text-lg font-semibold text-white">"Select Theme"</h2>
                        <button 
                            class="text-muted-foreground hover:text-white transition-colors"
                            on:click=move |_| show.set(false)
                        >
                            "✕"
                        </button>
                    </div>

                    // Theme Grid
                    <div class="p-4 overflow-y-auto grid grid-cols-3 gap-3">
                        {Theme::all().iter().map(|t| {
                            let theme = *t;
                            let is_active = move || active_theme.get() == theme;
                            
                            view! {
                                <button
                                    class=move || format!(
                                        "flex flex-col items-start p-2.5 rounded-theme border transition-all duration-200 group {}",
                                        if is_active() {
                                            "bg-primary/10 border-primary ring-1 ring-primary shadow-theme"
                                        } else {
                                            "bg-muted/20 border-border/5 hover:border-primary/40 hover:bg-muted/40"
                                        }
                                    )
                                    on:click=move |_| {
                                        active_theme.set(theme);
                                    }
                                >
                                    <div class="flex items-center gap-2 mb-2 w-full">
                                        <div 
                                            class="w-3.5 h-3.5 rounded-full border border-white/10 shrink-0"
                                            style=format!("background-color: {}", theme.preview_color())
                                        />
                                        <span class="text-xs font-medium text-white group-hover:text-primary transition-colors truncate">
                                            {theme.name()}
                                        </span>
                                    </div>
                                    // Mini preview UI
                                    <div class="w-full h-10 rounded-theme bg-black/40 border border-white/5 p-1.5 flex flex-col gap-1">
                                        <div class="w-1/2 h-1 rounded bg-white/20" />
                                        <div class="w-3/4 h-1 rounded bg-white/10" />
                                        <div class="mt-auto flex justify-between">
                                            <div class="w-4 h-1.5 rounded" style=format!("background-color: {}", theme.preview_color()) />
                                            <div class="w-2 h-2 rounded-full border border-white/20" />
                                        </div>
                                    </div>
                                </button>
                            }
                        }).collect_view()}
                    </div>

                    // Footer
                    <div class="p-4 border-t border-border/10 bg-muted/10 flex justify-end">
                        <button 
                            class="px-6 py-2 bg-primary text-primary-foreground font-semibold rounded-theme hover:opacity-90 transition-opacity"
                            on:click=move |_| show.set(false)
                        >
                            "Done"
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}

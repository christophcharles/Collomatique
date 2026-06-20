use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent, gtk,
};

use collomatique_strategies::StrategyProgress;

use crate::widgets::debug_view::{DebugView, DebugViewInput};

pub struct StrategyDisplay {
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum StrategyDisplayInput {
    Echo(String),
    Clear,
    StrategyUpdate(Result<StrategyProgress, String>),
    Finished,
}

#[relm4::component(pub)]
impl SimpleComponent for StrategyDisplay {
    type Init = ();
    type Input = StrategyDisplayInput;
    type Output = ();

    view! {
        #[root]
        gtk::Box {
            set_margin_all: 5,
            set_hexpand: true,
            set_vexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_margin_start: 5,
                set_label: "Informations de débogage :",
            },
            append = model.debug_view.widget(),
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let debug_view = DebugView::builder().launch(()).detach();

        let model = StrategyDisplay { debug_view };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            StrategyDisplayInput::Echo(line) => {
                self.debug_view
                    .emit(DebugViewInput::Append(format!("{line}\n")));
            }
            StrategyDisplayInput::Clear => {
                self.debug_view.emit(DebugViewInput::Clear);
            }
            StrategyDisplayInput::StrategyUpdate(progress) => {
                // TEMPORARY: route strategy progress to stderr until structured
                // UI reporting lands.
                match progress {
                    Ok(StrategyProgress::Default(p)) => {
                        eprintln!(
                            "  [strategy] obj={:.4} bound={:.4} nodes={} solutions={}",
                            p.best_obj, p.best_bound, p.node_count, p.solutions_found
                        );
                    }
                    Err(e) => eprintln!("  [strategy] [progress error] {e}"),
                }
            }
            StrategyDisplayInput::Finished => {}
        }
    }
}

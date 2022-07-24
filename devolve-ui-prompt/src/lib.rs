pub mod constr;
pub mod context;
pub mod resume;
mod prompt;
mod waker;
mod misc;

pub use prompt::*;

#[cfg(test)]
#[cfg(all(feature = "tui", feature = "time-blocking"))]
mod tests {
    use std::time::Duration;
    use crate::prompt::context::VPromptContext2;
    #[allow(unused_imports)]
    use crate::prompt::constr::{_make_prompt_macro, make_prompt, make_prompt_macro};
    use crate::component::node::VNode;
    use crate::hooks::BuiltinHooks;
    use crate::hooks::event::CallFirst;
    use crate::renderer::renderer::Renderer;
    use crate::view::layout::macros::smt;
    use crate::engines::tui::tui::{TuiConfig, TuiEngine};
    use crate::view_data::tui::constr::{vbox, text};
    use crate::view_data::tui::tui::HasTuiViewData;
    use test_log::test;
    use tokio::time::sleep;

    make_prompt!(pub my_component, MyComponentProps<ViewData: HasTuiViewData + Clone> {
        first: String = String::from("Untitled1"),
        second: String = String::from("Untitled2"),
        remaining: Vec<String> = vec![
            String::from("Untitled3"),
            String::from("Untitled4"),
            String::from("Untitled5"),
        ]
    } [children: Vec<VNode<ViewData>>]);

    async fn my_component<ViewData: HasTuiViewData + Clone + 'static>((mut c, wait_time): VPromptContext2<MyComponentProps<ViewData>, ViewData, Duration>) {
        c.yield_void(move |(mut c, mut resume, MyComponentProps { first, children, .. })| {
            // Can't move resume directly because it has a lifetime
            // However we can emulate any move by moving a state,
            // and then when we set the state we trigger an update.
            let do_resume = c.use_state(|_c| false);
            c.use_delay(wait_time, move |(mut c, _)| {
                c[do_resume] = true;
            });
            if c[do_resume] {
                resume.resume(()).expect("should've resumed first time");
                resume.resume(()).expect_err("should've failed resume second time");
            }

            vbox!({ width: smt!(100%) }, {}, vec![
                text!({}, {}, first.clone()),
                vbox!({}, {}, children.clone())
            ])
        }).await;

        let remaining = c.yield_(move |(_c, mut resume, MyComponentProps { second, remaining, .. })| {
            resume.resume(remaining.clone()).expect("should've resumed first time");
            resume.resume(vec![]).expect_err("should've failed resume second time");
            resume.resume(vec![]).expect_err("should've failed resume third time");

            vbox!({ width: smt!(100%) }, {}, vec![
                text!({}, {}, second.clone()),
            ])
        }).await;

        // TODO: This isn't tokio async so it crashes, figure out how to make it work
        sleep(wait_time).await;

        for next in remaining {
            c.yield_void(move |(mut c, mut resume, MyComponentProps { children, .. })| {
                // Can't move resume directly because it has a lifetime
                // However we can emulate any move by moving a state,
                // and then when we set the state we trigger an update.
                let do_resume = c.use_state(|_c| false);
                let did_resume = c.use_state(|_c| false);
                c.use_interval(wait_time, CallFirst::AfterTheInterval, move |(mut c, _)| {
                    c[do_resume] = true;
                });
                if c[do_resume] {
                    let resume_result = resume.resume(());
                    if !c[did_resume] {
                        resume_result.expect("should've resumed first time");
                        c[did_resume] = true;
                    } else {
                        resume_result.expect_err("shoudl've failed resume subsequent times");
                    }
                    c[do_resume] = false;
                }

                vbox!({ width: smt!(100%) }, {}, vec![
                    text!({}, {}, next.clone()),
                    vbox!({}, {}, children.clone())
                ])
            }).await;

            sleep(wait_time).await;
        }
    }

    #[test]
    fn test_component() {
        let renderer = Renderer::new(TuiEngine::new(TuiConfig::default()));
        renderer.root(|(mut c, ())| my_component!(c, "key", [Duration::from_secs(1)], { first: "Override title".to_owned() }, vec![
            text!({}, {}, "Hello world!".to_owned()),
        ]));
        renderer.show();
        renderer.resume_blocking();
    }
}
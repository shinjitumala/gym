//! The app shell: the routes, and the document the server renders.
//!
//! Three modes, each with a daily page; a mode's edit and analyse pages are
//! reached from the buttons at the bottom of its daily page. Food has no
//! analyse page: its intake chart lives under weight, beside the trend it is
//! read against.
//!
//! The stylesheet and the title are plain tags in `<head>` rather than
//! `leptos_meta` components. A meta component renders a placeholder where it
//! stands and moves the real element into the head, and the head rewrite on the
//! way to the browser leaves no placeholder behind — so the client side of
//! hydration looks for one and panics. Neither tag needs a signal, so neither
//! needs the meta machinery.

use crate::web::pages::*;
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="stylesheet" href="/pkg/gym.css" />
                <title>"gym"</title>
                <AutoReload options=options.clone() />
                <HydrationScripts options />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| "No such page.".into_view()>
                <Route path=StaticSegment("") view=FoodDailyPage />
                <Route path=StaticSegment("food") view=FoodDailyPage />
                <Route path=(StaticSegment("food"), StaticSegment("edit")) view=FoodEditPage />
                <Route path=StaticSegment("weight") view=WeightDailyPage />
                <Route path=(StaticSegment("weight"), StaticSegment("edit")) view=WeightEditPage />
                <Route
                    path=(StaticSegment("weight"), StaticSegment("analyse"))
                    view=WeightAnalysisPage
                />
                <Route path=StaticSegment("gym") view=GymDailyPage />
                <Route path=(StaticSegment("gym"), StaticSegment("edit")) view=GymEditPage />
                <Route
                    path=(StaticSegment("gym"), StaticSegment("analyse"))
                    view=GymAnalysisPage
                />
                <Route path=StaticSegment("status") view=StatusPage />
            </Routes>
        </Router>
    }
}

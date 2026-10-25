//! The app shell: the six routes, and the document the server renders.

use crate::web::pages::*;
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
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
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="gym" href="/pkg/gym.css" />
        <Title text="gym" />
        <Router>
            <Routes fallback=|| "No such page.".into_view()>
                <Route path=StaticSegment("") view=FoodDailyPage />
                <Route path=StaticSegment("food") view=FoodDailyPage />
                <Route path=(StaticSegment("food"), StaticSegment("edit")) view=FoodEditPage />
                <Route path=(StaticSegment("food"), StaticSegment("analysis")) view=FoodAnalysisPage />
                <Route path=StaticSegment("gym") view=GymDailyPage />
                <Route path=(StaticSegment("gym"), StaticSegment("edit")) view=GymEditPage />
                <Route path=(StaticSegment("gym"), StaticSegment("analysis")) view=GymAnalysisPage />
                <Route path=StaticSegment("status") view=StatusPage />
            </Routes>
        </Router>
    }
}

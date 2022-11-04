pub(crate) mod components;

use components::{
    navbar::Navbar,
    plugin::{PluginList, PluginSearch, PluginSearchIndex, PluginView},
    token::TokenList,
};
use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::view,
    reactive::{create_signal, provide_context_ref, ReadSignal, Scope},
    view::View,
    web::Html,
};
use sycamore_router::{HistoryIntegration, Route, Router};
use volts_core::MeUser;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Route)]
enum AppRoutes {
    #[to("/")]
    Index,
    #[to("/account")]
    Account,
    #[to("/plugins/<author>/<name>")]
    Plugin { author: String, name: String },
    #[to("/search/<query>")]
    Search { query: String },
    #[to("/search")]
    SearchIndex,
    #[not_found]
    NotFound,
}

#[derive(Clone, PartialEq, Eq, Default)]
pub struct AppContext {
    pub login: Option<String>,
}

#[component]
fn Index<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        PluginList {}
    }
}

#[component]
fn Account<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        TokenList {}
    }
}

#[wasm_bindgen(start)]
pub fn start_front() {
    console_error_panic_hook::set_once();
    sycamore::render(|cx| {
        view! { cx,
            Router(
                integration=HistoryIntegration::new(),
                view=|cx, route: &ReadSignal<AppRoutes>| {
                    let ctx = create_signal(cx, AppContext::default());
                    provide_context_ref(cx, ctx);

                    {
                        let req = Request::get("/api/v1/me").send();
                        sycamore::futures::spawn_local_scoped(cx, async move {
                            let resp = req.await.unwrap();
                            let resp: MeUser = resp.json().await.unwrap();
                            let mut new_ctx = (*ctx.get()).clone();
                            new_ctx.login = Some(resp.login);
                            ctx.set(new_ctx);
                        });
                    }

                    view! { cx,
                        Navbar {}
                        div {
                            (match route.get().as_ref() {
                                AppRoutes::Index => view! {cx,
                                    Index
                                },
                                AppRoutes::Account => view! {cx,
                                    Account
                                },
                                AppRoutes::Plugin { author, name } => view! {cx,
                                    PluginView(author=author.clone(), name=name.clone())
                                },
                                AppRoutes::Search { query } => view! {cx,
                                    PluginSearch(query=query.clone())
                                },
                                AppRoutes::SearchIndex => view! { cx,
                                    PluginSearchIndex
                                },
                                AppRoutes::NotFound => view! {cx,
                                    p(class="text-lg") {
                                        "404 Not Found"
                                    }
                                },
                            })
                        }
                    }
                }
            )
        }
    })
}

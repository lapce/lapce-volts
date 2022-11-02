use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::{view, Keyed},
    reactive::{create_signal, Scope, Signal},
    view::View,
    web::Html,
};
use volts_core::{EncodePlugin, PluginList};
use wasm_bindgen::{prelude::Closure, JsCast};
use web_sys::Event;

#[derive(PartialEq, Eq, Clone)]
struct IndexedPlugin {
    plugin: EncodePlugin,
    last: bool,
}

fn get_plugins<'a>(
    cx: Scope<'a>,
    q: Option<&str>,
    sort: Option<&str>,
    plugins: &'a Signal<Vec<IndexedPlugin>>,
) {
    let quries = &[("q", q), ("sort", sort)]
        .iter()
        .filter_map(|(name, value)| {
            let value = (*value)?;
            Some(format!("{name}={value}"))
        })
        .collect::<Vec<String>>()
        .join("&");
    let mut url = "/api/v1/plugins".to_string();
    if !quries.is_empty() {
        url = format!("{url}?{quries}");
    }

    let req = Request::get(&url).send();
    sycamore::futures::spawn_local_scoped(cx, async move {
        let resp = req.await.unwrap();
        let plugin_list: PluginList = resp.json().await.unwrap();
        let len = plugin_list.plugins.len();
        let plugin_list = plugin_list
            .plugins
            .into_iter()
            .enumerate()
            .map(|(i, plugin)| IndexedPlugin {
                plugin,
                last: i + 1 == len,
            })
            .collect();
        plugins.set(plugin_list);
    });
}

#[component(inline_props)]
fn PluginItem<'a, G: Html>(
    cx: Scope<'a>,
    plugin: IndexedPlugin,
    plugins: &'a Signal<Vec<IndexedPlugin>>,
) -> View<G> {
    let author = plugin.plugin.author.clone();
    let name = plugin.plugin.name.clone();
    let version = plugin.plugin.version.clone();

    let handle_img_error = move |event: Event| {
        let target: web_sys::HtmlImageElement = event.target().unwrap().unchecked_into();
        if target.src().ends_with("volt.png") {
            return;
        }
        target.set_src("/static/volt.png");
    };
    view! {cx,
        div(class="py-3") {
            li(
                class="flex border rounded-md py-4 w-full"
            ) {
                img(
                    class="m-4 h-auto w-16",
                    src=format!("/api/v1/plugins/{}/{}/{}/icon", author, name, version),
                    on:error=handle_img_error,
                ) {}
                div(class="flex flex-col justify-between w-[calc(100%-6rem)] pr-4") {
                    div {
                        p(
                            class="font-bold"
                        ) {
                            (plugin.plugin.display_name)
                        }
                        p(
                            class="mt-1 text-ellipsis whitespace-nowrap overflow-hidden"
                        ) {
                            (plugin.plugin.description)
                        }
                    }
                    div(
                        class="flex justify-between text-sm text-gray-400 mt-3"
                    ) {
                        p {
                            (plugin.plugin.author)
                        }
                        p {
                            "Downloads: " (plugin.plugin.downloads)
                        }
                    }
                }
            }
        }
    }
}

#[component(inline_props)]
fn PluginColumn<'a, G: Html>(cx: Scope<'a>, plugins: &'a Signal<Vec<IndexedPlugin>>) -> View<G> {
    view! {cx,
        ul {
            Keyed(
                iterable=plugins,
                view=move |cx, plugin| view! {cx,
                    PluginItem(plugin=plugin, plugins=plugins)
                },
                key=|plugin| plugin.plugin.name.clone(),
            )
        }
    }
}

#[component]
pub fn PluginList<G: Html>(cx: Scope) -> View<G> {
    let most_downloaded = create_signal(cx, Vec::new());
    let new_plugins = create_signal(cx, Vec::new());
    let recently_updated = create_signal(cx, Vec::new());
    get_plugins(cx, None, None, most_downloaded);
    get_plugins(cx, None, Some("created"), new_plugins);
    get_plugins(cx, None, Some("updated"), recently_updated);
    view! {cx,
        div(class="container m-auto") {
            div(class="flex flex-col items-center mt-16 mb-10 text-center") {
                h1(class="text-3xl mb-4") {
                    "Plugins for Lapce"
                }
                div(class="w-[36rem] max-w-full px-3") {
                    input(
                        class="border rounded-md text-lg w-full p-2",
                        placeholder="search lapce plugins"
                    ) {
                    }
                }
            }
            div(class="flex flex-wrap") {
                div(class="w-full px-3 lg:w-1/3") {
                    h1(class="mt-3 font-bold") {
                        "Most Downloaded"
                    }
                    PluginColumn(plugins=most_downloaded)
                }

                div(class="w-full px-3 lg:w-1/3") {
                    h1(class="mt-3 font-bold") {
                        "New Plugins"
                    }
                    PluginColumn(plugins=new_plugins)
                }

                div(class="w-full px-3 lg:w-1/3") {
                    h1(class="mt-3 font-bold") {
                        "Recently updated"
                    }
                    PluginColumn(plugins=recently_updated)
                }
            }
        }
    }
}

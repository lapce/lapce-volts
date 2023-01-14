use std::collections::HashMap;

use gloo_net::http::Request;
use sycamore::{
    component,
    prelude::{view, Keyed},
    reactive::{create_effect, create_selector, create_signal, Scope, Signal},
    view::View,
    web::Html,
};
use volts_core::{EncodePlugin, PluginList};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{Event, KeyboardEvent};

#[derive(PartialEq, Eq, Clone)]
struct IndexedPlugin {
    plugin: EncodePlugin,
    last: bool,
}

fn get_plugins<'a>(
    cx: Scope<'a>,
    q: Option<&str>,
    sort: Option<&str>,
    offset: Option<&str>,
    plugins: &'a Signal<Vec<IndexedPlugin>>,
    total: &'a Signal<i64>,
    loading: Option<&'a Signal<bool>>,
) {
    let quries = &[("q", q), ("sort", sort), ("offset", offset)]
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

    let offset: usize = offset.unwrap_or("0").parse().unwrap();
    let req = Request::get(&url).send();
    sycamore::futures::spawn_local_scoped(cx, async move {
        let resp = req.await.unwrap();
        let plugin_list: PluginList = resp.json().await.unwrap();
        let len = plugin_list.plugins.len();
        total.set(plugin_list.total);
        let plugin_list = plugin_list
            .plugins
            .into_iter()
            .enumerate()
            .map(|(i, plugin)| IndexedPlugin {
                plugin,
                last: i + 1 == len,
            });
        let mut current_plugins = (*plugins.get()).clone();
        current_plugins.truncate(offset);
        current_plugins.extend(plugin_list);
        plugins.set(current_plugins);
        if let Some(loading) = loading {
            loading.set(false);
        }
    });
}

#[component(inline_props)]
fn PluginItem<'a, G: Html>(
    cx: Scope<'a>,
    plugin: IndexedPlugin,
    plugins: &'a Signal<Vec<IndexedPlugin>>,
) -> View<G> {
    let author = create_signal(cx, plugin.plugin.author.clone());
    let name = create_signal(cx, plugin.plugin.name.clone());
    let version = plugin.plugin.version.clone();
    let updated_at = plugin.plugin.updated_at_ts;

    let handle_img_error = move |event: Event| {
        let target: web_sys::HtmlImageElement = event.target().unwrap().unchecked_into();
        if target.src().ends_with("volt.png") {
            return;
        }
        target.set_src("/static/volt.png");
    };
    view! {cx,
        div(class="py-3") {
            a(href=format!("/plugins/{}/{}", author.get(), name.get())) {
                li(
                    class="flex border rounded-md py-4 w-full"
                ) {
                    img(
                        class="m-4 h-16 w-16",
                        src=format!("/api/v1/plugins/{}/{}/{}/icon?id={}", author.get(), name.get(), version, updated_at),
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
                key=|plugin| plugin.plugin.id,
            )
        }
    }
}

#[component(inline_props)]
fn SearchInput<'a, G: Html>(
    cx: Scope<'a>,
    query: &'a Signal<String>,
    plugins: &'a Signal<Vec<IndexedPlugin>>,
    total: &'a Signal<i64>,
) -> View<G> {
    let jump_or_update = move || {
        if !web_sys::window()
            .unwrap()
            .location()
            .href()
            .unwrap()
            .as_str()
            .contains("/search")
        {
            web_sys::window()
                .unwrap()
                .location()
                .set_href(&format!("/search/{}", query.get()))
                .unwrap();
        } else {
            web_sys::window()
                .unwrap()
                .history()
                .unwrap()
                .push_state_with_url(
                    &JsValue::NULL,
                    "search",
                    Some(&format!("/search/{}", query.get())),
                )
                .unwrap();
            get_plugins(cx, Some(&query.get()), None, None, plugins, total, None);
        }
    };

    let handle_keyup = move |event: Event| {
        let event: KeyboardEvent = event.unchecked_into();
        if event.code() == "Enter" {
            jump_or_update();
        }
    };

    let handle_click = move |_| {
        jump_or_update();
    };

    view! {cx,
        div(class="w-[36rem] max-w-full flex items-center border rounded-md") {
            input(
                class="text-lg w-full p-2 pr-0",
                placeholder="search lapce plugins",
                on:keyup=handle_keyup,
                bind:value=query,
            ) {
            }
            button(
                class="p-3",
                on:click=handle_click,
            ) {
                svg(
                    class="h-4 w-4",
                    width=512,
                    height=512,
                    viewBox="0 0 512 512",
                    fill="gray",
                    xmlns="http://www.w3.org/2000/svg",
                ) {
                    path(
                        d="M505 442.7L405.3 343c-4.5-4.5-10.6-7-17-7H372c27.6-35.3 44-79.7 44-128C416 93.1 322.9 0 208 0S0 93.1 0 208s93.1 208 208 208c48.3 0 92.7-16.4 128-44v16.3c0 6.4 2.5 12.5 7 17l99.7 99.7c9.4 9.4 24.6 9.4 33.9 0l28.3-28.3c9.4-9.4 9.4-24.6.1-34zM208 336c-70.7 0-128-57.2-128-128 0-70.7 57.2-128 128-128 70.7 0 128 57.2 128 128 0 70.7-57.2 128-128 128z"
                    ) {}
                }
            }
        }
    }
}

#[component]
pub fn PluginList<G: Html>(cx: Scope) -> View<G> {
    let most_downloaded = create_signal(cx, Vec::new());
    let new_plugins = create_signal(cx, Vec::new());
    let recently_updated = create_signal(cx, Vec::new());
    let most_downloaded_total = create_signal(cx, 0);
    let new_plugins_total = create_signal(cx, 0);
    let recently_updated_total = create_signal(cx, 0);
    get_plugins(
        cx,
        None,
        None,
        None,
        most_downloaded,
        most_downloaded_total,
        None,
    );
    get_plugins(
        cx,
        None,
        Some("created"),
        None,
        new_plugins,
        new_plugins_total,
        None,
    );
    get_plugins(
        cx,
        None,
        Some("updated"),
        None,
        recently_updated,
        recently_updated_total,
        None,
    );

    let query = create_signal(cx, "".to_string());

    view! {cx,
        div(class="container m-auto") {
            div(class="flex flex-col items-center mt-16 mb-10 text-center") {
                h1(class="text-3xl mb-4") {
                    "Plugins for Lapce"
                }
                SearchInput(query=query, plugins=most_downloaded, total=most_downloaded_total)
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

#[component(inline_props)]
pub fn ReadmeView<'a, G: Html>(cx: Scope<'a>, text: &'a Signal<String>) -> View<G> {
    let markdown_html = create_signal(cx, "".to_string());
    create_effect(cx, || {
        let text = (*text.get()).to_string();
        let parser = pulldown_cmark::Parser::new_ext(&text, pulldown_cmark::Options::all());
        let mut html = "".to_string();
        pulldown_cmark::html::push_html(&mut html, parser);
        markdown_html.set(html);
    });
    view! { cx,
        (if text.get().is_empty() {
            view! {cx,
                p {"No Readme"}
            }
        } else {
            view! {cx,
                div(
                    class="prose prose-neutral",
                    dangerously_set_inner_html=&markdown_html.get(),
                )
            }
        })
    }
}

#[component(inline_props)]
pub fn PluginView<G: Html>(cx: Scope, author: String, name: String) -> View<G> {
    let plugin = create_signal(cx, None);
    let readme = create_signal(cx, "".to_string());

    let req = Request::get(&format!("/api/v1/plugins/{author}/{name}/latest")).send();
    sycamore::futures::spawn_local_scoped(cx, async move {
        let resp = req.await.unwrap();
        let resp: EncodePlugin = resp.json().await.unwrap();
        plugin.set(Some(resp.clone()));

        let req = Request::get(&format!(
            "/api/v1/plugins/{author}/{name}/{}/readme",
            resp.version
        ))
        .send();
        let resp = req.await.unwrap();
        if resp.status() == 200 {
            let resp = resp.text().await.unwrap();
            readme.set(resp);
        }
    });

    let handle_img_error = move |event: Event| {
        let target: web_sys::HtmlImageElement = event.target().unwrap().unchecked_into();
        if target.src().ends_with("volt.png") {
            return;
        }
        target.set_src("/static/volt.png");
    };

    view! {cx,
        (if plugin.get().is_none() {
            view! {cx,
            }
        } else {
            view! {cx,
                div(class="container m-auto mt-10") {
                    div(class="flex") {
                        img(
                            class="m-8 mt-2 h-24 w-24",
                            src=format!("/api/v1/plugins/{}/{}/{}/icon?id={}",
                                (*plugin.get()).as_ref().unwrap().author,
                                (*plugin.get()).as_ref().unwrap().name,
                                (*plugin.get()).as_ref().unwrap().version,
                                (*plugin.get()).as_ref().unwrap().updated_at_ts),
                            on:error=handle_img_error,
                        ) {}
                        div(
                            class="w-[calc(100%-10rem)]"
                        ) {
                            div(class="flex items-baseline") {
                                p(class="text-4xl font-bold") {
                                    ((*plugin.get()).as_ref().unwrap().display_name)
                                }
                                p(class="ml-4 px-2 rounded-md border bg-gray-200") {
                                    "v"((*plugin.get()).as_ref().unwrap().version)
                                }
                            }
                            p(class="text-lg mt-1") {
                                ((*plugin.get()).as_ref().unwrap().description)
                            }
                            div(class="flex mt-4 flex-wrap") {
                                p {
                                    ((*plugin.get()).as_ref().unwrap().author)
                                }
                                p(class="ml-4") {
                                    "｜"
                                }
                                p(class="ml-4") {
                                    ((*plugin.get()).as_ref().unwrap().downloads) " Downloads"
                                }
                            }
                        }
                    }
                    hr(class="my-8 h-px bg-gray-200 border-0") {}
                    div(class="flex flex-wrap") {
                        div(class="w-full lg:w-2/3 px-10") {
                            ReadmeView(text=readme)
                        }
                        div(class="w-full lg:w-1/3 mt-8 lg:mt-0 px-10 lg:px-4") {
                            p(class="font-bold") {
                                "Repository"
                            }
                            div(class="mt-2") {
                                (if (*plugin.get()).as_ref().unwrap().repository.is_none() {
                                    view!{cx, p {""}}
                                } else {
                                    view!{cx,
                                        a(
                                            class="text-blue-500 hover:text-blue-700",
                                            target="_blank",
                                            href=(*plugin.get()).as_ref().unwrap().repository.clone().unwrap(),
                                        ) {
                                            ((*plugin.get()).as_ref().unwrap().repository.clone().unwrap())
                                        }
                                    }
                                })
                            }

                            p(class="font-bold mt-8") {
                                "More Information"
                            }
                            p(class="mt-2") {
                                table(class="table-auto") {
                                    tbody {
                                        tr {
                                            td {
                                                "Version"
                                            }
                                            td {
                                                ((*plugin.get()).as_ref().unwrap().version)
                                            }
                                        }
                                        tr {
                                            td {
                                                "Author"
                                            }
                                            td {
                                                ((*plugin.get()).as_ref().unwrap().author)
                                            }
                                        }
                                        tr {
                                            td(class="pr-4") {
                                                "Released"
                                            }
                                            td {
                                                ((*plugin.get()).as_ref().unwrap().released_at)
                                            }
                                        }
                                        tr {
                                            td(class="pr-4") {
                                                "Last Updated"
                                            }
                                            td {
                                                ((*plugin.get()).as_ref().unwrap().updated_at)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}

#[component(inline_props)]
pub fn PluginSearch<G: Html>(cx: Scope, query: String) -> View<G> {
    let query = create_signal(cx, query);
    let plugins = create_signal(cx, Vec::new());
    let plugins_total = create_signal(cx, 0);
    get_plugins(
        cx,
        Some(&query.get()),
        None,
        None,
        plugins,
        plugins_total,
        None,
    );

    let loading_more = create_signal(cx, false);

    let handle_scroll = move |event: Event| {
        if *loading_more.get() {
            return;
        }
        if plugins.get().len() == *plugins_total.get() as usize {
            return;
        }
        web_sys::console::log_1(
            &format!(
                "plugins len {}, total {}",
                plugins.get().len(),
                plugins_total.get()
            )
            .into(),
        );
        let target: web_sys::HtmlElement = event.target().unwrap().unchecked_into();
        let scroll_height = target.scroll_height();
        let scroll_top = target.scroll_top();
        let client_height = target.client_height();

        if scroll_height - scroll_top - client_height < 50 {
            loading_more.set(true);
            let offset = plugins.get().len().to_string();
            get_plugins(
                cx,
                Some(&query.get()),
                None,
                Some(&offset),
                plugins,
                plugins_total,
                Some(loading_more),
            );
            web_sys::console::log_1(&format!("loading more now").into());
        }
    };

    let is_plugins_empty = create_selector(cx, || plugins.get().is_empty());

    view! { cx,
        div(class="container m-auto") {
            div(class="flex flex-col items-center mt-10 mb-6 text-center") {
                SearchInput(query=query, plugins=plugins, total=plugins_total)
            }
            (if *is_plugins_empty.get() {
                view! {cx,
                    div(class="flex flex-col items-center mt-3 text-center") {
                        p {
                            "0 Plugins Found"
                        }
                    }
                }
            } else {
                view! { cx,
                    div(
                        class="overflow-y-scroll h-[calc(100vh-16rem)]",
                        on:scroll=handle_scroll,
                    ) {
                        ul(
                            class="px-3",
                        ) {
                            Keyed(
                                iterable=plugins,
                                view=move |cx, plugin| view! {cx,
                                    PluginItem(plugin=plugin, plugins=plugins)
                                },
                                key=|plugin| plugin.plugin.id,
                            )
                        }
                    }
                }
            })
        }
    }
}

#[component]
pub fn PluginSearchIndex<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        PluginSearch(query="".to_string())
    }
}

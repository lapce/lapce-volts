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

#[component(inline_props)]
pub fn PluginView<G: Html>(cx: Scope, author: String, name: String) -> View<G> {
    let plugin = create_signal(cx, None);

    let req = Request::get(&format!("/api/v1/plugins/{author}/{name}/latest")).send();
    sycamore::futures::spawn_local_scoped(cx, async move {
        let resp = req.await.unwrap();
        let resp: EncodePlugin = resp.json().await.unwrap();
        plugin.set(Some(resp));
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
                                    ((*plugin.get()).as_ref().unwrap().downloads) " Dowloads"
                                }
                            }
                        }
                    }
                    hr(class="my-8 h-px bg-gray-200 border-0") {}
                    div(class="flex flex-wrap") {
                        div(class="w-full lg:w-2/3 px-10") {
                            p {
                                "No readme"
                            }
                        }
                        div(class="w-full lg:w-1/3 px-4") {
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
